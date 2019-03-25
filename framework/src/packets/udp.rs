/*
* Copyright 2019 Comcast Cable Communications Management, LLC
*
* Licensed under the Apache License, Version 2.0 (the "License");
* you may not use this file except in compliance with the License.
* You may obtain a copy of the License at
*
* http://www.apache.org/licenses/LICENSE-2.0
*
* Unless required by applicable law or agreed to in writing, software
* distributed under the License is distributed on an "AS IS" BASIS,
* WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
* See the License for the specific language governing permissions and
* limitations under the License.
*
* SPDX-License-Identifier: Apache-2.0
*/

use std::fmt;
use packets::{Fixed, Header, Packet};
use packets::ip::IpPacket;

/*  From (https://tools.ietf.org/html/rfc768)
    User Datagram Header Format

     0      7 8     15 16    23 24    31
    +--------+--------+--------+--------+
    |     Source      |   Destination   |
    |      Port       |      Port       |
    +--------+--------+--------+--------+
    |                 |                 |
    |     Length      |    Checksum     |
    +--------+--------+--------+--------+
    |
    |          data octets ...
    +---------------- ...

    Source Port is an optional field, when meaningful, it indicates the port
    of the sending  process,  and may be assumed  to be the port  to which a
    reply should  be addressed  in the absence of any other information.  If
    not used, a value of zero is inserted.

    Destination  Port has a meaning  within  the  context  of  a  particular
    internet destination address.

    Length  is the length  in octets  of this user datagram  including  this
    header  and the data.   (This  means  the minimum value of the length is
    eight.)

    Checksum is the 16-bit one's complement of the one's complement sum of a
    pseudo header of information from the IP header, the UDP header, and the
    data,  padded  with zero octets  at the end (if  necessary)  to  make  a
    multiple of two octets.

    The pseudo  header  conceptually prefixed to the UDP header contains the
    source  address,  the destination  address,  the protocol,  and the  UDP
    length.   This information gives protection against misrouted datagrams.
    This checksum procedure is the same as is used in TCP.

                 0      7 8     15 16    23 24    31
                +--------+--------+--------+--------+
                |          source address           |
                +--------+--------+--------+--------+
                |        destination address        |
                +--------+--------+--------+--------+
                |  zero  |protocol|   UDP length    |
                +--------+--------+--------+--------+

    If the computed  checksum  is zero,  it is transmitted  as all ones (the
    equivalent  in one's complement  arithmetic).   An all zero  transmitted
    checksum  value means that the transmitter  generated  no checksum  (for
    debugging or for higher level protocols that don't care).
*/

/// UDP header
#[derive(Debug, Default, Copy, Clone)]
#[repr(C)]
pub struct UdpHeader {
    src_port: u16,
    dst_port: u16,
    length: u16,
    checksum: u16
}

impl Header for UdpHeader {}

/// UDP packet
pub struct Udp<E: IpPacket> {
    envelope: E,
    mbuf: *mut MBuf,
    offset: usize,
    header: *mut UdpHeader
}

impl<E: IpPacket> Udp<E> {
    #[inline]
    pub fn src_port(&self) -> u16 {
        u16::from_be(self.header().src_port)
    }

    #[inline]
    pub fn set_src_port(&mut self, src_port: u16) {
        self.header().src_port = u16::to_be(src_port);
    }

    #[inline]
    pub fn dst_port(&self) -> u16 {
        u16::from_be(self.header().dst_port)
    }

    #[inline]
    pub fn set_dst_port(&mut self, dst_port: u16) {
        self.header().dst_port = u16::to_be(dst_port);
    }

    #[inline]
    pub fn length(&self) -> u16 {
        u16::from_be(self.header().length)
    }

    #[inline]
    pub fn set_length(&mut self, length: u16) {
        self.header().length = u16::to_be(length);
    }

    #[inline]
    pub fn checksum(&self) -> u16 {
        u16::from_be(self.header().checksum)
    }

    #[inline]
    pub fn set_checksum(&mut self, checksum: u16) {
        self.header().checksum = u16::to_be(checksum);
    }
}

impl<E: IpPacket> fmt::Display for Udp<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "src_port: {}, dst_port: {}, length: {}, checksum: {}",
            self.src_port(),
            self.dst_port(),
            self.length(),
            self.checksum()
        )
    }
}

impl<E: IpPacket> Packet for Udp<E> {
    type Envelope = E;
    type Header = UdpHeader;

    #[inline]
    fn from_packet(envelope: Self::Envelope,
                   mbuf: *mut MBuf,
                   offset: usize,
                   header: *mut Self::Header) -> Result<Self> {
        Ok(Udp {
            envelope,
            mbuf,
            offset,
            header
        })
    }

    #[inline]
    fn envelope(&self) -> &Self::Envelope {
        &self.envelope
    }

    #[inline]
    fn mbuf(&self) -> *mut MBuf {
        self.mbuf
    }

    #[inline]
    fn offset(&self) -> usize {
        self.offset
    }

    #[inline]
    fn header(&self) -> &mut Self::Header {
        unsafe { &mut (*self.header) }
    }

    #[inline]
    fn header_len(&self) -> usize {
        Self::Header::size()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use dpdk_test;
    use packets::{Ethernet, RawPacket};
    use packets::ip::v4::Ipv4;

    #[rustfmt::skip]
    pub const UDP_PACKET: [u8; 52] = [
        // ** ethernet header
        0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
        0x08, 0x00,
        // ** IPv4 header
        0x45, 0x00,
        // IPv4 payload length
        0x00, 0x26,
        // ident = 43849, flags = 4, frag_offset = 0
        0xab, 0x49, 0x40, 0x00,
        // ttl = 255, protocol = UDP, checksum = 0xf700
        0xff, 0x11, 0xf7, 0x00,
        // src = 139.133.217.110
        0x8b, 0x85, 0xd9, 0x6e,
        // dst = 139.133.233.2
        0x8b, 0x85, 0xe9, 0x02,
        // ** UDP header
        // src_port = 39376, dst_port = 1087
        0x99, 0xd0, 0x04, 0x3f,
        // UDP length = 18, checksum = 0x7228
        0x00, 0x12, 0x72, 0x28,
        // ** UDP payload
        0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x68, 0x65, 0x6c, 0x6c, 0x6f
    ];

    #[test]
    fn size_of_udp_header() {
        assert_eq!(8, UdpHeader::size());
    }

    #[test]
    fn parse_udp_packet() {
        dpdk_test! {
            let packet = RawPacket::from_bytes(&UDP_PACKET).unwrap();
            let ethernet = packet.parse::<Ethernet>().unwrap();
            let ipv4 = ethernet.parse::<Ipv4>().unwrap();
            let udp = ipv4.parse::<Udp<Ipv4>>().unwrap();

            assert_eq!(39376, udp.src_port());
            assert_eq!(1087, udp.dst_port());
            assert_eq!(18, udp.length());
            assert_eq!(0x7228, udp.checksum());
        }
    }
}
