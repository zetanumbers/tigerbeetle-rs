use std::{mem, ptr, sync::Mutex};

use tokio::sync::{Semaphore as AsyncSemaphore, SemaphorePermit as AsyncSemaphorePermit};

use crate::sys;

pub struct ResourcePool {
    free_packets: Mutex<sys::tb_packet_list_t>,
    packet_count: AsyncSemaphore,
}

impl ResourcePool {
    pub fn new(free_packets: sys::tb_packet_list_t, packet_count: usize) -> Self {
        Self {
            free_packets: Mutex::new(free_packets),
            packet_count: AsyncSemaphore::new(packet_count),
        }
    }

    pub async fn acquire_packet(&self) -> PacketGuard<'_> {
        PacketGuard {
            _permit: self
                .packet_count
                .acquire()
                .await
                .expect("acquiring permit from packet_count semaphore"),
            packet: {
                let mut free_packets = self
                    .free_packets
                    .lock()
                    .expect("free_packets mutex is poisoned");
                let Some(mut head) = ptr::NonNull::new(free_packets.head) else {
                    panic!("no free packets despite acquired permit")
                };
                // SAFETY: referring exclusively from this mutable borrow
                let new_head = unsafe { mem::replace(&mut head.as_mut().next, ptr::null_mut()) };
                if new_head.is_null() {
                    free_packets.head = ptr::null_mut();
                    free_packets.tail = ptr::null_mut();
                } else {
                    free_packets.head = new_head;
                }

                head
            },
            free_packets: &self.free_packets,
        }
    }
}

pub struct PacketGuard<'a> {
    packet: ptr::NonNull<sys::tb_packet_t>,
    _permit: AsyncSemaphorePermit<'a>,
    free_packets: &'a Mutex<sys::tb_packet_list_t>,
}

impl PacketGuard<'_> {
    pub fn packet(&self) -> ptr::NonNull<sys::tb_packet_t> {
        self.packet
    }
}

impl Drop for PacketGuard<'_> {
    fn drop(&mut self) {
        let Ok(mut free_packets) = self
            .free_packets
            .lock()
            else { return };

        let packet = self.packet.as_ptr();
        if free_packets.tail.is_null() {
            free_packets.head = packet;
            free_packets.tail = packet;
        } else {
            let tail = mem::replace(&mut free_packets.tail, packet);
            unsafe { (*tail).next = packet };
        }
    }
}
