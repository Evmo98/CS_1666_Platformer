use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::str::FromStr;
use crate::player::Player;
use crate::object_controller::ObjectController;

pub const PACKET_SIZE: usize = 80;
const DEBUG: bool = false;

#[derive(Copy, Clone)]
pub enum Mode {
    MultiplayerPlayer1,
    MultiplayerPlayer2,
}

pub struct Multiplayer {
    pub mode: Mode,
}

pub struct Connection {
    pub send_socket: UdpSocket,
    pub receive_socket: UdpSocket,
}

impl Connection {
    pub fn new(mode: Mode) -> Connection {
        /*
        if on windows, use the PowerShell command
        Get-NetIPAddress -InterfaceAlias Wi-Fi | select IPAddress
        on p1's machine and p2's machine. Then change the addresses below
        accordingly but don't change the port numbers (the numbers after the ':')
        */
        let p1_address = IpAddr::from_str("127.0.0.1").unwrap();
        let p2_address = IpAddr::from_str("127.0.0.1").unwrap();
        let connected_socket = |local, remote| {
            let socket = UdpSocket::bind(local).expect("couldn't bind to local");
            socket.connect(remote).expect("couldn't connect to remote");
            socket
        };
        match mode {
            Mode::MultiplayerPlayer1 => {
                let send_socket = connected_socket (
                    SocketAddr::new(p1_address, 34254),
                    SocketAddr::new(p2_address, 34256),
                );
                let receive_socket = connected_socket(
                    SocketAddr::new(p1_address, 34255),
                    SocketAddr::new(p2_address, 34257)
                );
                Connection { send_socket, receive_socket }
            }
            Mode::MultiplayerPlayer2 => {
                let send_socket = connected_socket (
                    SocketAddr::new(p2_address, 34257),
                    SocketAddr::new(p1_address, 34255),
                );
                let receive_socket = connected_socket(
                    SocketAddr::new(p2_address, 34256),
                    SocketAddr::new(p1_address, 34254)
                );
                Connection { send_socket, receive_socket }
            }
        }
    }
}

impl Multiplayer {
    pub fn new(mode: Mode) -> Multiplayer {
        Multiplayer { mode }
    }
}

pub fn pack_data(
    player: &mut Player,
    block: &ObjectController,
    multiplayer: &Option<Multiplayer>,
) -> Vec<u8> {

    //Player Information
    let player_xpos = player.physics.x().to_le_bytes();
    let player_ypos = player.physics.y().to_le_bytes();
    let flip = player.flip_horizontal as u32;
    let flip = flip.to_le_bytes();
    let anim = player.anim.next_anim(multiplayer);
    let ax = anim.x().to_le_bytes();
    let ay = anim.y().to_le_bytes();
    let aw = anim.width().to_le_bytes();
    let ah = anim.height().to_le_bytes();

    //Portal Information
    let portal_x: [u8; 4];
    let portal_y: [u8; 4];
    let portal_rotation: [u8; 4];

    match multiplayer.as_ref().unwrap().mode {
        Mode::MultiplayerPlayer1 => {
            portal_x = player.portal.portals[0].x().to_le_bytes();
            portal_y = player.portal.portals[0].y().to_le_bytes();
            portal_rotation = player.portal.portals[0].rotation().to_le_bytes();
        },
        Mode::MultiplayerPlayer2 => {
            portal_x = player.portal.portals[1].x().to_le_bytes();
            portal_y = player.portal.portals[1].y().to_le_bytes();
            portal_rotation = player.portal.portals[1].rotation().to_le_bytes();
        }
    }

    //Block Information
    let block_x: [u8; 4] = block.x().to_le_bytes();
    let block_y: [u8; 4] = block.y().to_le_bytes();
    let carried = block.carried as u32;
    let block_carried: [u8; 4] = carried.to_le_bytes();

    //Wand Information
    let wand_x: [u8; 4] = player.portal.wand_x().to_le_bytes();
    let wand_y: [u8; 4] = player.portal.wand_y().to_le_bytes();
    let wand_rotation: [u8; 4] = player.portal.rotation().to_le_bytes();

    // Potion Information
    let potion_state = player.portal.get_potion_state();
    let which_potion: i32 = if potion_state.0.is_some() { 0 } else if potion_state.1.is_some() { 1 } else { 2 };
    let (x, y, r) =
        match which_potion {
            0 => { potion_state.0.unwrap() },
            1 => { potion_state.1.unwrap() },
            _ => {(0 as f32, 0 as f32, 0 as f64)}
        };
    let r = r as f32;
    let potion_x: [u8; 4] = x.to_le_bytes();
    let potion_y: [u8; 4] = y.to_le_bytes();
    let potion_rotation: [u8; 4] = r.to_le_bytes();
    let which_potion: [u8; 4] = which_potion.to_le_bytes();

    let buf = [
        player_xpos,
        player_ypos,
        flip,
        portal_x,
        portal_y,
        portal_rotation,
        ax,
        ay,
        aw,
        ah,
        block_x,
        block_y,
        block_carried,
        wand_x,
        wand_y,
        wand_rotation,
        potion_x,
        potion_y,
        potion_rotation,
        which_potion,
    ].concat();
    if DEBUG { println!("{:?}", &buf); }

    buf
}

pub fn recv_packet_buffer(socket: UdpSocket) -> Result<[u8; PACKET_SIZE], String> {
    let mut buf: [u8; PACKET_SIZE] = [0; PACKET_SIZE];
    let receive_result = socket.recv(&mut buf);
    return match receive_result {
        Ok(_) => {
            let amt = receive_result.unwrap();
            if amt != PACKET_SIZE {
                eprintln!("Expected {} bytes, Received {} bytes", PACKET_SIZE, amt);
            }
            Ok(buf)
        }
        Err(_) => {
            Err(String::from("Didn't receive data"))
        }
    }
}

pub fn unpack_player_data(buf: &mut [u8; PACKET_SIZE])
                          -> Result<(f32, f32, bool, i32, i32, u32, u32), String> {
    let mut xpos: [u8; 4] = [0; 4];
    for i in 0..4 {
        xpos[i] = buf[i];
    }

    let mut ypos: [u8; 4] = [0; 4];
    for i in 4..8 {
        ypos[i-4] = buf[i];
    }
    let mut flip: [u8; 4] = [0; 4];
    for i in 8..12 {
        flip[i-8] = buf[i];
    }

    let mut ax :[u8; 4] = [0; 4];
    for i in 24..28 {
        ax[i-24] = buf[i];
    }

    let mut ay :[u8; 4] = [0; 4];
    for i in 28..32 {
        ay[i-28] = buf[i];
    }

    let mut aw :[u8; 4] = [0; 4];
    for i in 32..36 {
        aw[i-32] = buf[i];
    }

    let mut ah :[u8; 4] = [0; 4];
    for i in 36..40 {
        ah[i-36] = buf[i];
    }

    let x = f32::from_le_bytes(xpos);
    let y = f32::from_le_bytes(ypos);
    let flip = u32::from_le_bytes(flip);
    if flip != 1 && flip != 0 {
        return Err(String::from("Error: player flip is neither 1 nor 0"));
    }
    let flip = flip == 1;
    let ax = i32::from_le_bytes(ax);
    let ay = i32::from_le_bytes(ay);
    let aw = u32::from_le_bytes(aw);
    let ah = u32::from_le_bytes(ah);
    // debug
    let tup = (x, y, flip, ax, ay, aw, ah);
    if DEBUG {
        println!("tup = {:?}", tup);
        println!("buf = {:?}", buf);
    }
    Ok(tup)
}

// refactor to make safe -- return result
pub fn unpack_portal_data(buf: &mut [u8; PACKET_SIZE]) -> (f32, f32, f32) {
    let mut xpos: [u8; 4] = [0; 4];
    for i in 12..16 {
        xpos[i-12] = buf[i];
    }

    let mut ypos: [u8; 4] = [0; 4];
    for i in 16..20 {
        ypos[i-16] = buf[i];
    }

    let mut rotation: [u8; 4] = [0; 4];
    for i in 20..24 {
        rotation[i-20] = buf[i];
    }

    let x1 = f32::from_le_bytes(xpos);
    let y1 = f32::from_le_bytes(ypos);
    let rotation1 = f32::from_le_bytes(rotation);
   
    (x1,y1,rotation1)
}

pub(crate) fn unpack_block_data(buf: &mut [u8; PACKET_SIZE]) -> (i32, i32, bool){
    let mut block_x: [u8; 4] = [0; 4];
    for i in 40..44 {
        block_x[i-40] = buf[i];
    }

    let mut block_y: [u8; 4] = [0; 4];
    for i in 44..48 {
        block_y[i-44] = buf[i];
    }

    let mut carried: [u8; 4] = [0; 4];
    for i in 48..52 {
        carried[i-48] = buf[i];
    }

    let block_x = i32::from_le_bytes(block_x);
    let block_y = i32::from_le_bytes(block_y);
    let carried = i32::from_le_bytes(carried);
    let carried = match carried {
        0 => false,
        _ => true,
    };

    (block_x, block_y, carried)
}

pub(crate) fn unpack_wand_data(buf: &mut [u8; PACKET_SIZE]) -> (i32, i32, f32) {
    let mut wand_x: [u8; 4] = [0; 4];
    for i in 52..56 {
        wand_x[i-52] = buf[i];
    }

    let mut wand_y: [u8; 4] = [0; 4];
    for i in 56..60 {
        wand_y[i-56] = buf[i];
    }

    let mut wand_rot: [u8; 4] = [0; 4];
    for i in 60..64 {
        wand_rot[i-60] = buf[i];
    }

    let wand_x = i32::from_le_bytes(wand_x);
    let wand_y = i32::from_le_bytes(wand_y);
    let wand_rot = f32::from_le_bytes(wand_rot);

    (wand_x, wand_y, wand_rot)
}

pub(crate) fn unpack_potion_data(buf: &mut [u8; PACKET_SIZE]) -> (f32, f32, f32, i32) {
    let mut potion_x: [u8; 4] = [0; 4];
    for i in 64..68 {
        potion_x[i-64] = buf[i];
    }

    let mut potion_y: [u8; 4] = [0; 4];
    for i in 68..72 {
        potion_y[i-68] = buf[i];
    }

    let mut potion_rot: [u8; 4] = [0; 4];
    for i in 72..76 {
        potion_rot[i-72] = buf[i];
    }

    let mut potion_state: [u8; 4] = [0; 4];
    for i in 76..80 {
        potion_state[i-76] = buf[i];
    }

    let potion_x = f32::from_le_bytes(potion_x);
    let potion_y = f32::from_le_bytes(potion_y);
    let potion_rot = f32::from_le_bytes(potion_rot);
    let potion_state = i32::from_le_bytes(potion_state);

    (potion_x, potion_y, potion_rot, potion_state)
}
