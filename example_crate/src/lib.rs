use byond_fn::byond_fn;

#[byond_fn]
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[byond_fn]
pub fn add_optional(left: usize, right: Option<usize>) -> usize {
    left + right.unwrap_or(0)
}
