use alloc::rc::Rc;
use core::cell::RefCell;

use crate::factory::RrosElement;
use kernel::{
    bindings, container_of,
    irq_work::IrqWork,
    pr_debug,
    workqueue::{init_work, queue_work_on, Work},
};

pub struct RrosWork {
    irq_work: IrqWork,
    wq_work: Work,
    wq: *mut bindings::workqueue_struct,
    pub handler: Option<fn(arg: &mut RrosWork) -> i32>,
    // element : Rc<RefCell<RrosElement>>
    element: Option<Rc<RefCell<RrosElement>>>,
}

fn do_wq_work(wq_work: *mut Work) {
    let work = container_of!(wq_work, RrosWork, wq_work);
    let handler = unsafe { (*work).handler.unwrap() };
    let work = unsafe { &mut *(work as *mut RrosWork) };
    handler(work);

    // TODO:
    // if (work->element)
    // rros_put_element(work->element);
}

unsafe extern "C" fn do_irq_work(irq_work: *mut IrqWork) {
    let work = container_of!(irq_work, RrosWork, irq_work) as *mut RrosWork;
    if unsafe {
        !queue_work_on(
            bindings::WORK_CPU_UNBOUND as _,
            (*work).wq,
            &mut (*work).wq_work,
        ) && (*work).element.is_some()
    } {
        pr_debug!("uncompleted rros_put_element()");
    }
    // TODO: rros_put_element is not implemented
    // if unsafe{rust_helper_queue_work((*work).wq,&mut (*work).wq_work)} && unsafe{(*)}
    // if (!queue_work(work->wq, &work->wq_work) && work->element)
    // rros_put_element(work->element);
}

impl RrosWork {
    pub const fn new() -> Self {
        unsafe {
            core::mem::transmute::<[u8; core::mem::size_of::<Self>()], Self>(
                [0; core::mem::size_of::<Self>()],
            )
        }
        // RrosWork{
        //     element : None,
        //     // element: Rc::try_new(RefCell::new(RrosElement::new().unwrap())).unwrap(),
        //     handler : None,
        //     wq : core::ptr::null_mut(),
        //     wq_work : bindings::work_struct{
        //         data : bindings::atomic64_t { counter: 0 },
        //         entry : bindings::list_head{
        //             next : core::ptr::null_mut(),
        //             prev : core::ptr::null_mut(),
        //         },
        //         // func : Some(0 as extern "C" fn(*mut bindings::work_struct)),
        //         func: None
        //     },
        //     irq_work : IrqWork::new()
        // }
    }
    pub fn init(&mut self, handler: fn(arg: &mut RrosWork) -> i32) {
        let _ret = self.irq_work.init_irq_work(do_irq_work);
        init_work(&mut self.wq_work, do_wq_work);
        self.handler = Some(handler);
        self.element = Some(Rc::try_new(RefCell::new(RrosElement::new().unwrap())).unwrap());
    }
    pub fn init_safe(
        &mut self,
        handler: fn(arg: &mut RrosWork) -> i32,
        element: Rc<RefCell<RrosElement>>,
    ) {
        let _ret = self.irq_work.init_irq_work(do_irq_work);
        init_work(&mut self.wq_work, do_wq_work);
        self.handler = Some(handler);
        self.element = Some(element);
    }
    pub fn call_inband_from(&mut self, wq: *mut bindings::workqueue_struct) {
        self.wq = wq;
        // TODO: rros_put_element is not implemented
        // if (work->element)
        if self.element.is_some() {
            pr_debug!("uncompleted rros_get_element()");
        }
        // rros_get_element(work->element);
        if self.irq_work.irq_work_queue().is_err() && self.element.is_some() {
            pr_debug!("uncompleted rros_put_element()")
        }
        // if (!irq_work_queue(&work->irq_work) && work->element)
        // rros_put_element(work->element);
        // unsafe{rust_helper_queue_work(wq,&mut self.wq_work)};
    }

    #[inline]
    pub fn call_inband(&mut self) {
        self.call_inband_from(unsafe { bindings::system_wq });
    }
}
