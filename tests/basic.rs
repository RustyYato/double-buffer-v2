#[test]
fn basic_swap() {
    let mut inner = double_buffer::base::Inner::new(double_buffer::strategy::local::LocalStrategy::default(), 0, 1);
    let (mut w, mut r) = double_buffer::base::new(&mut inner);
    assert_eq!(*w.get(), 0);
    assert_eq!(*r.get(), 1);
    w.swap_buffers();
    assert_eq!(*w.get(), 1);
    assert_eq!(*r.get(), 0);
}

#[test]
#[should_panic(expected = "Could not swap buffers")]
fn swap_with_active_readers() {
    let mut inner = double_buffer::base::Inner::new(double_buffer::strategy::local::LocalStrategy::default(), 0, 1);
    let (mut w, mut r) = double_buffer::base::new(&mut inner);
    let _rg = r.get();
    w.swap_buffers();
}
