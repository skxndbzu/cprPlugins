use std::{process::Stdio, time::Duration};

use gateway_plugin_sdk::{
    CallContext, Frame, Handshake, Manifest, Message, Stage,
    client::{read_frame, write_frame},
};
use serde_json::{Value, json};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

struct Host {
    child: Child,
    reader: ChildStdout,
    writer: ChildStdin,
}

impl Host {
    async fn start(extra: Value) -> Self {
        let manifest = Manifest::from_author_slice(include_bytes!("../plugin.json")).unwrap();
        let binary = std::env::var_os("BASISPOINTS_TEST_BINARY")
            .unwrap_or_else(|| env!("CARGO_BIN_EXE_cpr-plugin-oai-basispoints").into());
        let mut child = Command::new(binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut host = Self {
            reader: child.stdout.take().unwrap(),
            writer: child.stdin.take().unwrap(),
            child,
        };
        let mut configuration = json!({"models":["gpt-6-astra"]});
        configuration
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        host.send(
            Message::Hello {
                handshake: Handshake {
                    protocol_version: 1,
                    artifact_sha256: "a".repeat(64),
                    plugin_id: "local.oai-basispoints".into(),
                    instance_id: "test".into(),
                    generation: 1,
                    incarnation: "inc".into(),
                    configuration,
                    permissions: manifest.permissions.into_iter().collect(),
                    contributes: manifest.contributes,
                },
            },
            vec![],
        )
        .await;
        assert!(matches!(
            host.receive().await.message,
            Message::Ready { .. }
        ));
        host.call(1, "plugin.register", Stage::Registration, json!({}), vec![])
            .await;
        assert!(matches!(
            host.receive().await.message,
            Message::Result { id: 1, .. }
        ));
        host
    }

    async fn send(&mut self, message: Message, payload: Vec<u8>) {
        write_frame(&mut self.writer, &Frame { message, payload })
            .await
            .unwrap();
    }

    async fn receive(&mut self) -> Frame {
        tokio::time::timeout(Duration::from_secs(5), read_frame(&mut self.reader))
            .await
            .expect("timeout")
            .expect("valid SDK frame")
    }

    async fn call(&mut self, id: u64, method: &str, stage: Stage, params: Value, payload: Vec<u8>) {
        self.send(
            Message::Call {
                id,
                method: method.into(),
                context: CallContext {
                    call_id: id,
                    instance_id: "test".into(),
                    generation: 1,
                    incarnation: "inc".into(),
                    stage,
                    timeout_ms: 4000,
                    resource_scope_id: format!("scope-{id}"),
                    request_id: if stage == Stage::Attempt {
                        Some("request-test".into())
                    } else {
                        None
                    },
                    attempt_id: None,
                    account_id: (stage == Stage::Attempt).then(|| "acct_test".into()),
                    credential_revision: None,
                },
                params,
            },
            payload,
        )
        .await;
    }

    async fn request(&mut self, source: Value) {
        let params = json!({"request_id":"request-test","mount":"attempt","attempt_index":1,"provider":"openai","account_id":"acct_test","operation":"generate","protocol":"openai","endpoint":"/v1/responses","transport":if source["stream"]==true {"http_sse"} else {"http_json"},"model":source["model"],"body_visible":true,"headers":[]});
        self.call(
            3,
            "middleware.handle",
            Stage::Attempt,
            params,
            source.to_string().into_bytes(),
        )
        .await;
        self.send(
            Message::Credit {
                id: 3,
                bytes: 256 * 1024,
                frames: 32,
            },
            vec![],
        )
        .await;
    }

    async fn callback(
        &mut self,
        expected: &str,
        result: Value,
        payload: Vec<u8>,
    ) -> (Value, Vec<u8>) {
        let frame = self.receive().await;
        let Message::Callback {
            id,
            parent_id: 3,
            method,
            params,
        } = frame.message
        else {
            panic!("expected callback, got {:?}", frame.message)
        };
        assert_eq!(method, expected);
        self.send(Message::Result { id, result }, payload).await;
        (params, frame.payload)
    }
}

#[tokio::test]
async fn selected_model_delegates_once_without_credentials_or_http_callbacks() {
    for (model, groups, expected) in [
        (
            "gpt-6-astra",
            vec!["grp_00000000000000000000000000000001"],
            Some("openai-basispoints-v2"),
        ),
        ("other", vec!["grp_00000000000000000000000000000001"], None),
        ("gpt-6-astra", vec![], None),
    ] {
        let mut host = Host::start(json!({"account_group_ids":groups})).await;
        host.request(json!({"model":model,"input":"hello","stream":false}))
            .await;
        let (params, payload) = host.callback("host.middleware.next", json!({"response":"host-response","protocol":"openai","status":200,"headers":[],"body":null}), vec![]).await;
        assert_eq!(params["protocol"].as_str(), expected);
        if expected.is_some() {
            assert_eq!(params["body"], "replace");
            let body: Value = serde_json::from_slice(&payload).unwrap();
            assert_eq!(body["_cpr_basispoints_group_ids"], json!(groups));
            assert_eq!(body["input"], "hello");
            assert_eq!(body["model"], model);
        } else {
            assert_eq!(params["body"], "preserve");
            assert!(payload.is_empty());
        }
        let frame = host.receive().await;
        let Message::Result { id: 3, result } = frame.message else {
            panic!("expected result")
        };
        assert_eq!(result["response"], "host-response");
        assert_eq!(result["body"]["kind"], "empty");
        let frame = host.receive().await;
        assert!(matches!(frame.message, Message::End { id: 3, error: None }));
        host.send(Message::Shutdown, vec![]).await;
        assert!(host.child.wait().await.unwrap().success());
    }
}
