use std::ptr;

use tokio::sync::SemaphorePermit as AsyncSemaphorePermit;

use crate::{sys, Client};

pub struct PacketGuard {
    packet: ptr::NonNull<sys::tb_packet_t>,
    _permit: AsyncSemaphorePermit<'static>,
    // drop this only after `_permit` is droped
    client: Client,
}

impl PacketGuard {
    pub async fn acquire(client: Client) -> Self {
        let permit = client.shared.pool.acquire().await.unwrap();
        let mut packet = ptr::null_mut();
        let status = unsafe { sys::tb_client_acquire_packet(client.shared.raw, &mut packet) };
        assert_eq!(status, sys::TB_PACKET_ACQUIRE_STATUS::TB_PACKET_ACQUIRE_OK);
        PacketGuard {
            packet: ptr::NonNull::new(packet).unwrap(),
            // SAFETY: owning a client with the semaphore
            _permit: unsafe { std::mem::transmute(permit) },
            client,
        }
    }

    pub fn packet(&self) -> ptr::NonNull<sys::tb_packet_t> {
        self.packet
    }
}

impl Drop for PacketGuard {
    fn drop(&mut self) {
        unsafe { sys::tb_client_release_packet(self.client.shared.raw, self.packet.as_ptr()) }
    }
}

fn _test_thread_safety(packet: &mut PacketGuard) {
    check_thread_safety(&mut packet._permit);
    check_thread_safety(&mut packet.client);
    fn check_thread_safety<T: Send + Sync>(_: T) {}
}
