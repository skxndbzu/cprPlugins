//! 使用目标宿主的真实包校验器与 RPC 运行时验证安装包，不连接数据库或真实模型服务。
use futures::future::BoxFuture;
use gateway_plugin_runtime::{
    CallbackHandler, PackageLimits, RpcLimits, RpcReply, RpcSession, ValidatedPackage,
};
use gateway_plugin_sdk::{CallContext, Handshake, PluginFault, Stage};
use serde_json::{Value, json};
use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

const GROUP: &str = "grp_00000000000000000000000000000001";

struct Callbacks {
    expected_bps: bool,
    calls: Arc<AtomicUsize>,
}
impl CallbackHandler for Callbacks {
    fn call(
        &self,
        context: CallContext,
        method: String,
        params: Value,
        payload: Vec<u8>,
    ) -> BoxFuture<'static, Result<RpcReply, PluginFault>> {
        assert_eq!(method, "host.middleware.next");
        assert_eq!(context.account_id.as_deref(), Some("acct_compat"));
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.expected_bps {
            assert_eq!(params["protocol"], "openai-basispoints-v2");
            assert_eq!(params["body"], "replace");
            let body: Value = serde_json::from_slice(&payload).unwrap();
            assert_eq!(body["_cpr_basispoints_group_ids"], json!([GROUP]));
            assert_eq!(body["input"], "compatibility fixture");
            assert_eq!(body["model"], "gpt-6-astra");
        } else {
            assert_eq!(params["body"], "preserve");
            assert!(params.get("protocol").is_none_or(Value::is_null));
            assert!(payload.is_empty());
        }
        Box::pin(async {
            Ok(RpcReply {
                result: json!({"response":"host-response","protocol":"openai","status":200,"headers":[],"body":null}),
                payload: vec![],
            })
        })
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let archive = std::fs::read(&args[1]).unwrap();
    let host_version = args[2].parse().unwrap();
    let package =
        Arc::new(ValidatedPackage::read(archive.into(), None, PackageLimits::default()).unwrap());
    let cache = tempfile::tempdir().unwrap();
    if args.get(3).is_some_and(|arg| arg == "reject") {
        let error = package
            .prepare(cache.path(), &host_version)
            .err()
            .expect("old package must reject unsupported host");
        assert!(matches!(
            error,
            gateway_plugin_runtime::PackageError::Manifest(
                gateway_plugin_sdk::ManifestError::Platform
            )
        ));
        println!("Confirmed manifest rejects host {host_version}");
        return;
    }
    let prepared = Arc::new(package.prepare(cache.path(), &host_version).unwrap());
    let processes = gateway_host::process::ProcessSupervisor::new(NonZeroUsize::new(8).unwrap());
    for (model, groups, expected_bps) in [
        ("gpt-6-astra", vec![GROUP], true),
        ("other", vec![GROUP], false),
        ("gpt-6-astra", vec![], false),
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let session = Arc::new(
            RpcSession::start(
                Arc::clone(&prepared),
                Handshake {
                    protocol_version: gateway_plugin_sdk::PROTOCOL_VERSION,
                    artifact_sha256: package.digest().into(),
                    plugin_id: package.manifest().plugin_id().unwrap(),
                    instance_id: "compat-test".into(),
                    generation: 1,
                    incarnation: uuid::Uuid::new_v4().to_string(),
                    configuration: json!({"models":["gpt-6-astra"],"account_group_ids":groups}),
                    contributes: package.manifest().contributes.clone(),
                    permissions: package.manifest().permissions.iter().copied().collect(),
                },
                RpcLimits::default(),
                &processes,
                Arc::new(Callbacks {
                    expected_bps,
                    calls: Arc::clone(&calls),
                }),
            )
            .await
            .unwrap(),
        );
        let registration = session
            .call(
                "plugin.register",
                session.context(Stage::Registration, Duration::from_secs(10)),
                json!({}),
                vec![],
            )
            .await
            .unwrap();
        assert_eq!(
            registration.result["contributes"],
            serde_json::to_value(&package.manifest().contributes).unwrap()
        );
        // 同时覆盖 JSON 与 SSE 入口；插件应保留宿主返回的响应句柄。
        for stream in [false, true] {
            let mut context = session.context(Stage::Attempt, Duration::from_secs(10));
            context.request_id = Some("req-compat".into());
            context.account_id = Some("acct_compat".into());
            let params = json!({"request_id":"req-compat","mount":"attempt","attempt_index":1,"provider":"openai","account_id":"acct_compat","operation":"generate","protocol":"openai","endpoint":"/v1/responses","transport":if stream {"http_sse"} else {"http_json"},"model":model,"body_visible":true,"headers":[]});
            let body = serde_json::to_vec(
                &json!({"model":model,"input":"compatibility fixture","stream":stream}),
            )
            .unwrap();
            let mut response = session
                .call_stream("middleware.handle", context, params, body)
                .await
                .unwrap();
            assert_eq!(response.initial.result["response"], "host-response");
            assert_eq!(response.initial.result["body"]["kind"], "empty");
            assert!(response.next().await.unwrap().is_none());
        }
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        session.shutdown(Duration::from_secs(2)).await;
        assert!(!session.is_ready());
    }
    println!(
        "PASS host {host_version}: package hashes/platform/version, real handshake/registration, six middleware calls, group/model passthrough, response identity, shutdown"
    );
}
