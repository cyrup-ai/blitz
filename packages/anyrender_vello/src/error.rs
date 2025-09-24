use thiserror::Error;

#[derive(Error, Debug)]
pub enum TextRenderError {
    #[error("Glyphon prepare failed: {0}")]
    PrepareFailed(#[from] glyphon::PrepareError),

    #[error("Glyphon render failed: {0}")]
    RenderFailed(#[from] glyphon::RenderError),

    #[error("Font system unavailable")]
    FontSystemUnavailable,

    #[error("Invalid text bounds")]
    InvalidBounds,

    #[error("Text atlas allocation failed")]
    AtlasAllocationFailed,

    #[error("GPU resource creation failed: {0}")]
    GpuResourceError(String),
}

pub type TextRenderResult<T> = Result<T, TextRenderError>;
