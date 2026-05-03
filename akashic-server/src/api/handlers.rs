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
    State(_state): State<AppState>,
    Json(_request): Json<CreateGameSessionRequest>,
) -> ApiResult<CreateGameSessionData> {
    Err(AppError::not_implemented("POST /api/game-sessions"))
}

pub async fn get_game_session(
    State(_state): State<AppState>,
    Path(_path): Path<SessionPath>,
) -> ApiResult<GameSessionSnapshot> {
    Err(AppError::not_implemented(
        "GET /api/game-sessions/:sessionId",
    ))
}

pub async fn submit_choice(
    State(_state): State<AppState>,
    Path(_path): Path<SessionPath>,
    Json(_request): Json<SubmitChoiceRequest>,
) -> ApiResult<SubmitChoiceData> {
    Err(AppError::not_implemented(
        "POST /api/game-sessions/:sessionId/choices",
    ))
}

pub async fn get_game_session_ending(
    State(_state): State<AppState>,
    Path(_path): Path<SessionPath>,
) -> ApiResult<GameSessionEndingData> {
    Err(AppError::not_implemented(
        "GET /api/game-sessions/:sessionId/ending",
    ))
}

pub async fn create_intuition_preview(
    State(_state): State<AppState>,
    Path(_path): Path<SessionPath>,
    Json(_request): Json<IntuitionPreviewRequest>,
) -> ApiResult<IntuitionPreviewData> {
    Err(AppError::not_implemented(
        "POST /api/game-sessions/:sessionId/intuition-preview",
    ))
}

pub async fn get_game_session_history(
    State(_state): State<AppState>,
    Path(_path): Path<SessionPath>,
) -> ApiResult<HistoryListData> {
    Err(AppError::not_implemented(
        "GET /api/game-sessions/:sessionId/history",
    ))
}

pub async fn stream_game_session(
    State(_state): State<AppState>,
    Path(_path): Path<SessionPath>,
) -> ApiResult<StreamHandshakeData> {
    Err(AppError::not_implemented(
        "GET /api/game-sessions/:sessionId/stream",
    ))
}

pub async fn create_save(
    State(_state): State<AppState>,
    Json(_request): Json<CreateSaveRequest>,
) -> ApiResult<SaveSummary> {
    Err(AppError::not_implemented("POST /api/saves"))
}

pub async fn list_saves(State(_state): State<AppState>) -> ApiResult<SaveListData> {
    Err(AppError::not_implemented("GET /api/saves"))
}

pub async fn load_save(
    State(_state): State<AppState>,
    Path(_path): Path<SavePath>,
) -> ApiResult<LoadSaveData> {
    Err(AppError::not_implemented("POST /api/saves/:saveId/load"))
}

pub async fn list_archives(State(_state): State<AppState>) -> ApiResult<ArchiveListData> {
    Err(AppError::not_implemented("GET /api/archives"))
}

pub async fn get_archive(
    State(_state): State<AppState>,
    Path(_path): Path<ArchivePath>,
) -> ApiResult<ArchiveDetailData> {
    Err(AppError::not_implemented("GET /api/archives/:archiveId"))
}

pub async fn generate_save_share_card(
    State(_state): State<AppState>,
    Json(_request): Json<GenerateSaveShareCardRequest>,
) -> ApiResult<ShareCardData> {
    Err(AppError::not_implemented("POST /api/share/save-card"))
}

pub async fn generate_ending_share_card(
    State(_state): State<AppState>,
    Json(_request): Json<GenerateEndingShareCardRequest>,
) -> ApiResult<ShareCardData> {
    Err(AppError::not_implemented("POST /api/share/ending-card"))
}
