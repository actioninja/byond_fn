use byond_fn::byond_fn;

#[byond_fn]
pub fn add(left: usize, right: usize) -> usize {
    println!("Where did you come from, where did you go?");
    left + right
}
