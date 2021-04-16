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

#[test]
#[cfg(feature = "alloc")]
fn basic_op_writer() {
    struct Op(i32);

    impl double_buffer::traits::Operation<i32> for Op {
        fn apply(&mut self, buffer: &mut i32) { *buffer += self.0 }
    }

    let mut inner =
        double_buffer::base::Inner::new(double_buffer::strategy::saving::SavingStrategy::default(), 0, 0);
    let (w, mut r) = double_buffer::base::new(&mut inner);
    let mut w = double_buffer::op::OpWriter::from(w);
    assert_eq!(*r.get(), 0);
    w.push(Op(-2));
    assert_eq!(*r.get(), 0);
    w.swap_buffers();
    assert_eq!(*r.get(), -2);
    w.push(Op(2));
    assert_eq!(*r.get(), -2);
    w.swap_buffers();
    assert_eq!(*r.get(), 0);
    w.push(Op(2));
    assert_eq!(*r.get(), 0);
    w.swap_buffers();
    assert_eq!(*r.get(), 2);
    w.swap_buffers();
    assert_eq!(*r.get(), 2);
    w.swap_buffers();
    assert_eq!(*r.get(), 2);
}

#[test]
fn infinite() {
    let mut inner = double_buffer::base::Inner::new(double_buffer::strategy::sync::SyncStrategy::INIT, (), ());
    let (mut w, mut r) = double_buffer::base::new(&mut inner);
    w.swap_buffers();
    let _rg = r.get();
    let swap = unsafe { w.start_buffer_swap() };
    drop(_rg);
    w.finish_buffer_swap(swap);
}
