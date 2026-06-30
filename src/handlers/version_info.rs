use crate::built_info;
use axum::Json;
use chimitheque_types::versioninfo::VersionInfo;

pub async fn get_version_info() -> Json<VersionInfo> {
    Json(VersionInfo {
        version: built_info::PKG_VERSION.to_owned(),
        build_time: built_info::BUILT_TIME_UTC.to_owned(),
        git_commit: Some(built_info::GIT_COMMIT_HASH.unwrap_or_default().to_string()),
        git_commit_hash: Some(built_info::GIT_COMMIT_HASH.unwrap_or_default().to_string()),
        target: built_info::TARGET.to_owned(),
        rustc: built_info::RUSTC_VERSION.to_owned(),
    })
}
