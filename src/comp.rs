use crate::impls::comp::comp_dict::CompDict;

/// BENCHMARK ONLY, DO NOT USE
#[doc(hidden)]
pub fn create_comp_dict(data: &[u8]) {
    unsafe {
        let mut dict = CompDict::new(data);
        dict.get_item(0, 0, 0);
    };
}
