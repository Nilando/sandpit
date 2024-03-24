#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AllocError {
    BadRequest,
    OOM,
}

impl From<BlockError> for AllocError {
    fn from(error: BlockError) -> AllocError {
        match error {
            BlockError::BadRequest => AllocError::BadRequest,
            BlockError::OOM => AllocError::OOM,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum BlockError {
    BadRequest,
    OOM,
}
