// Necessary because of this issue: https://github.com/rust-lang/cargo/issues/9641
fn main() {
  embuild::espidf::sysenv::output();
}
