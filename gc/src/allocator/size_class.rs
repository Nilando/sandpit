use super::constants;
use super::errors::AllocError;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SizeClass {
    Small,
    Medium,
    Large,
}

impl SizeClass {
    pub fn get_for_size(object_size: usize) -> Result<SizeClass, AllocError> {
        match object_size {
            constants::SMALL_OBJECT_MIN..=constants::SMALL_OBJECT_MAX => Ok(SizeClass::Small),
            constants::MEDIUM_OBJECT_MIN..=constants::MEDIUM_OBJECT_MAX => Ok(SizeClass::Medium),
            constants::LARGE_OBJECT_MIN..=constants::LARGE_OBJECT_MAX => Ok(SizeClass::Large),
            _ => Err(AllocError::BadRequest),
        }
    }

    pub const fn const_get_for_size(object_size: usize) -> SizeClass {
        match object_size {
            constants::SMALL_OBJECT_MIN..=constants::SMALL_OBJECT_MAX => SizeClass::Small,
            constants::MEDIUM_OBJECT_MIN..=constants::MEDIUM_OBJECT_MAX => SizeClass::Medium,
            constants::LARGE_OBJECT_MIN..=constants::LARGE_OBJECT_MAX => SizeClass::Large,
            _ => panic!("An AllocObject's size is too large!")
        }
    }
}
