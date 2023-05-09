use byond_fn::byond_fn;

#[byond_fn]
pub fn add(left: usize, right: usize) -> usize {
    left + right
}
