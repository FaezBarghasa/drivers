use std::fs::File;

use pcid_interface::*;

use crate::{transport::Error, Device};

pub fn enable_msix(_pcid_handle: &mut PciFunctionHandle) -> Result<File, Error> {
    Err(Error::Disabled)
}
