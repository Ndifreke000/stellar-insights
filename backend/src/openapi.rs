use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(),
    components()
)]
pub struct ApiDoc;