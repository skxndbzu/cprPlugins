use cpr_plugin_oai_basispoints::adapter::Config;
use serde_json::json;
#[test]
fn requires_model_list_and_rejects_legacy_credential_configuration() {
    assert!(Config::from_value(json!({"models":["gpt-6-astra"]})).is_ok());
    assert!(Config::from_value(json!({})).unwrap().models.is_empty());
    assert!(Config::from_value(json!({"account_id":"acct_test"})).is_err());
    assert!(Config::from_value(json!({"models":{"alias":"target"}})).is_err());
    assert!(Config::from_value(json!({"account_group_ids":["group name"]})).is_err());
    assert!(
        Config::from_value(json!({"account_group_ids":["grp_00000000000000000000000000000001"]}))
            .is_ok()
    );
}
