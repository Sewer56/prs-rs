/// BENCHMARK ONLY, DO NOT USE
#[doc(hidden)]
pub fn create_comp_dict(data: &[u8]) {
    use crate::impls::comp::comp_dict::CompDict;

    unsafe {
        let mut dict = CompDict::create(data);
        dict.get_item(0, 0, 0);
    };
}
