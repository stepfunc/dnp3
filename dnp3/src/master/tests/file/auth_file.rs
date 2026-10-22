use crate::app::{FunctionCode, Group70Var2, Sequence};
use crate::master::association::AssociationConfig;
use crate::master::tests::harness::{create_association, AssocInfoEvent};
use crate::master::{FileCredentials, TaskType};

#[tokio::test]
async fn reports_file_auth_task_type() {
    let config = AssociationConfig::quiet();
    let mut harness = create_association(config).await;

    let mut association = harness.association.clone();
    let auth_task = tokio::spawn(async move {
        association
            .get_file_auth_key(FileCredentials {
                user_name: "user".to_string(),
                password: "pass".to_string(),
            })
            .await
    });

    harness
        .expect_write(super::request(
            FunctionCode::AuthenticateFile,
            0,
            &Group70Var2 {
                auth_key: 0,
                user_name: "user",
                password: "pass",
            },
        ))
        .await;

    harness
        .process_response(super::response(
            0,
            &Group70Var2 {
                auth_key: 42,
                user_name: "",
                password: "",
            },
        ))
        .await;

    assert_eq!(u32::from(auth_task.await.unwrap().unwrap()), 42);

    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[
            AssocInfoEvent::TaskStart(
                TaskType::FileAuth,
                FunctionCode::AuthenticateFile,
                Sequence::new(0)
            ),
            AssocInfoEvent::TaskSuccess(
                TaskType::FileAuth,
                FunctionCode::AuthenticateFile,
                Sequence::new(0)
            ),
        ]
    );
}
