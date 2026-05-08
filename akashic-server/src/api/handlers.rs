use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    error::AppError,
    state::AppState,
};

use super::dto::{
    ApiResponse, ArchiveDetailData, ArchiveListData, ArchivePath, CreateGameSessionData,
    CreateGameSessionRequest, CreateSaveRequest, GameSessionEndingData, GameSessionSnapshot,
    GenerateEndingShareCardRequest, GenerateSaveShareCardRequest, HealthzData, HistoryListData,
    IntuitionPreviewData, IntuitionPreviewRequest, LoadSaveData, SaveListData, SavePath,
    SaveSummary, SessionPath, ShareCardData, StreamHandshakeData, SubmitChoiceData,
    SubmitChoiceRequest,
};

type ApiResult<T> = Result<Json<ApiResponse<T>>, AppError>;

pub async fn healthz(State(state): State<AppState>) -> Json<ApiResponse<HealthzData>> {
    Json(ApiResponse::ok(HealthzData {
        status: "ok".to_string(),
        service_name: state.service_name.to_string(),
        api_version: state.api_version.to_string(),
    }))
}

pub async fn create_game_session(
    State(state): State<AppState>,
    Json(request): Json<CreateGameSessionRequest>,
) -> ApiResult<CreateGameSessionData> {
    Ok(Json(ApiResponse::ok(state.create_game_session(request)?)))
}

pub async fn get_game_session(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
) -> ApiResult<GameSessionSnapshot> {
    Ok(Json(ApiResponse::ok(state.get_game_session(&path.session_id)?)))
}

pub async fn submit_choice(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
    Json(request): Json<SubmitChoiceRequest>,
) -> ApiResult<SubmitChoiceData> {
    Ok(Json(ApiResponse::ok(
        state.submit_choice(&path.session_id, request)?,
    )))
}

pub async fn get_game_session_ending(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
) -> ApiResult<GameSessionEndingData> {
    Ok(Json(ApiResponse::ok(
        state.get_game_session_ending(&path.session_id)?,
    )))
}

pub async fn create_intuition_preview(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
    Json(request): Json<IntuitionPreviewRequest>,
) -> ApiResult<IntuitionPreviewData> {
    Ok(Json(ApiResponse::ok(
        state.create_intuition_preview(&path.session_id, &request.choice_id)?,
    )))
}

pub async fn get_game_session_history(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
) -> ApiResult<HistoryListData> {
    Ok(Json(ApiResponse::ok(
        state.get_game_session_history(&path.session_id)?,
    )))
}

pub async fn stream_game_session(
    State(state): State<AppState>,
    Path(path): Path<SessionPath>,
) -> ApiResult<StreamHandshakeData> {
    Ok(Json(ApiResponse::ok(
        state.stream_game_session(&path.session_id)?,
    )))
}

pub async fn create_save(
    State(state): State<AppState>,
    Json(request): Json<CreateSaveRequest>,
) -> ApiResult<SaveSummary> {
    Ok(Json(ApiResponse::ok(state.create_save(request)?)))
}

pub async fn list_saves(State(state): State<AppState>) -> ApiResult<SaveListData> {
    Ok(Json(ApiResponse::ok(state.list_saves())))
}

pub async fn load_save(
    State(state): State<AppState>,
    Path(path): Path<SavePath>,
) -> ApiResult<LoadSaveData> {
    Ok(Json(ApiResponse::ok(state.load_save(&path.save_id)?)))
}

pub async fn list_archives(State(state): State<AppState>) -> ApiResult<ArchiveListData> {
    Ok(Json(ApiResponse::ok(state.list_archives())))
}

pub async fn get_archive(
    State(state): State<AppState>,
    Path(path): Path<ArchivePath>,
) -> ApiResult<ArchiveDetailData> {
    Ok(Json(ApiResponse::ok(state.get_archive(&path.archive_id)?)))
}

pub async fn generate_save_share_card(
    State(state): State<AppState>,
    Json(request): Json<GenerateSaveShareCardRequest>,
) -> ApiResult<ShareCardData> {
    Ok(Json(ApiResponse::ok(
        state.generate_save_share_card(&request.save_id, &request.style)?,
    )))
}

pub async fn generate_ending_share_card(
    State(state): State<AppState>,
    Json(request): Json<GenerateEndingShareCardRequest>,
) -> ApiResult<ShareCardData> {
    Ok(Json(ApiResponse::ok(
        state.generate_ending_share_card(&request.archive_id, &request.style)?,
    )))
}
