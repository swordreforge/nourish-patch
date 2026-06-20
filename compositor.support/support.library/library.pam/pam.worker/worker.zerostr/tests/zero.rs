use compositor_support_library_pam_worker_zerostr::ZeroString;

#[test]
fn basic_ops() {
    let mut z = ZeroString::new();
    assert!(z.is_empty());
    z.push('h');
    z.push('i');
    assert_eq!(z.as_str(), "hi");
    assert_eq!(z.len(), 2);
    z.clear();
    assert!(z.is_empty());
}

#[test]
fn debug_does_not_leak() {
    let mut z = ZeroString::new();
    z.push('s');
    z.push('e');
    z.push('c');
    let s = format!("{:?}", z);
    assert!(s.contains("REDACTED"));
    assert!(!s.contains("sec"));
}

#[test]
fn clone_is_independent() {
    let mut a = ZeroString::new();
    a.push('x');
    let b = a.clone();
    a.clear();
    assert_eq!(b.as_str(), "x");
    assert!(a.is_empty());
}

#[test]
fn deref_to_str() {
    let mut z = ZeroString::new();
    z.push('a');
    z.push('b');
    let s: &str = &z;
    assert_eq!(s, "ab");
}
