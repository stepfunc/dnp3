use crate::app::{FileStatus, FunctionCode, Group70Var2, Sequence};
use crate::master::association::AssociationConfig;
use crate::master::tests::harness::{create_association, AssocInfoEvent, TestHarness};
use crate::master::{AuthKey, FileCredentials, FileError, TaskError, TaskType};

#[tokio::test]
async fn reports_file_auth_task_type() {
    let config = AssociationConfig::quiet();
    let mut harness = create_association(config).await;

    let auth_task = spawn_auth_task(&harness);

    harness.expect_write(auth_request(0)).await;

    harness.process_response(auth_response(0, 42)).await;

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

#[tokio::test]
async fn auth_request_bytes() {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let _auth_task = spawn_auth_task(&harness);
    harness.expect_write(AUTH_REQUEST.to_vec()).await;
}

#[tokio::test]
async fn fails_with_no_permission_when_auth_key_is_zero() {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let auth_task = spawn_auth_task(&harness);
    harness.expect_write(auth_request(0)).await;
    harness.process_response(auth_response(0, 0)).await;

    assert_eq!(
        auth_task.await.unwrap().map(u32::from),
        Err(FileError::NoPermission)
    );
    expect_failure(&mut harness, TaskError::UnexpectedResponseHeaders);
}

#[tokio::test]
async fn fails_on_unexpected_variation() {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let auth_task = spawn_auth_task(&harness);
    harness.expect_write(auth_request(0)).await;
    harness
        .process_response(super::file_status_response(0, 1, 0, 0, FileStatus::Success))
        .await;

    assert_eq!(
        auth_task.await.unwrap().map(u32::from),
        Err(FileError::BadResponse)
    );
    expect_failure(&mut harness, TaskError::UnexpectedResponseHeaders);
}

#[tokio::test]
async fn fails_without_object_header() {
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let auth_task = spawn_auth_task(&harness);
    harness.expect_write(auth_request(0)).await;
    harness.process_response(super::no_object_response(0)).await;

    assert_eq!(
        auth_task.await.unwrap().map(u32::from),
        Err(FileError::TaskError(TaskError::UnexpectedResponseHeaders))
    );
    expect_failure(&mut harness, TaskError::UnexpectedResponseHeaders);
}

/// AUTHENTICATE_FILE request for user "user" and password "pass" with sequence 0
pub(super) const AUTH_REQUEST: &[u8] = &[
    0xC0, 0x1D, 70, 2, 0x5B, 0x01, 0x14, 0x00, // header, length = 20
    0x0C, 0x00, 0x04, 0x00, // user name offset and size
    0x10, 0x00, 0x04, 0x00, // password offset and size
    0x00, 0x00, 0x00, 0x00, // auth key
    b'u', b's', b'e', b'r', b'p', b'a', b's', b's',
];

fn expect_failure(harness: &mut TestHarness, err: TaskError) {
    assert_eq!(
        harness.assoc_events.pop().as_slice(),
        &[
            AssocInfoEvent::TaskStart(
                TaskType::FileAuth,
                FunctionCode::AuthenticateFile,
                Sequence::new(0)
            ),
            AssocInfoEvent::TaskFailure(TaskType::FileAuth, err),
        ]
    );
}

fn spawn_auth_task(harness: &TestHarness) -> tokio::task::JoinHandle<Result<AuthKey, FileError>> {
    let mut association = harness.association.clone();
    tokio::spawn(async move {
        association
            .get_file_auth_key(FileCredentials {
                user_name: "user".to_string(),
                password: "pass".to_string(),
            })
            .await
    })
}

pub(super) fn auth_request(seq: u8) -> Vec<u8> {
    super::request(
        FunctionCode::AuthenticateFile,
        seq,
        &Group70Var2 {
            auth_key: 0,
            user_name: "user",
            password: "pass",
        },
    )
}

pub(super) fn auth_response(seq: u8, auth_key: u32) -> Vec<u8> {
    super::response(
        seq,
        &Group70Var2 {
            auth_key,
            user_name: "",
            password: "",
        },
    )
}
