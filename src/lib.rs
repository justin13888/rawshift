fn hello_world() -> &'static str {
    "Hello World!"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        assert_eq!(hello_world(), "Hello World!");
    }
}
