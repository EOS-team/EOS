// use crate::list;

// extern "C" {
//     fn rust_helper_INIT_LIST_HEAD(list: *mut list::ListHead);
//     fn rust_helper_list_del(list: *mut list::ListHead);
// }

// pub fn rros_init_schedq(q: Rc<RefCell<sched::RrosSchedQueue>>) {
//     let mut q_ptr = q.borrow_mut();
//     init_list_head(&mut q_ptr.head as *mut list::ListHead);
// }

// fn init_list_head(list: *mut list::ListHead) {
//     unsafe { rust_helper_INIT_LIST_HEAD(list) };
// }

// pub fn rros_get_schedq(struct RrosSchedQueue *q){
// 	if (list_empty(&q->head))
// 		return NULL;

// 	return list_get_entry(&q->head, struct RrosThread, rq_next);
// }
