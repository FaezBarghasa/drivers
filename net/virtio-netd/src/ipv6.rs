
#[derive(Debug, Clone, Copy)]
pub struct Ipv6 {
    pub link_local: [u8; 16],
    pub global: [u8; 16],
    pub unique_local: [u8; 16],
}

impl Ipv6 {
    pub fn new(mac_address: [u8; 6], global: [u8; 16], unique_local: [u8; 16]) -> Self {
        let mut link_local: [u8; 16] = [0; 16];
        link_local[0] = 0xfe;
        link_local[1] = 0x80;
        link_local[8] = mac_address[0] ^ 0x02;
        link_local[9] = mac_address[1];
        link_local[10] = mac_address[2];
        link_local[11] = 0xff;
        link_local[12] = 0xfe;
        link_local[13] = mac_address[3];
        link_local[14] = mac_address[4];
        link_local[15] = mac_address[5];

        Self {
            link_local,
            global,
            unique_local,
        }
    }
}
