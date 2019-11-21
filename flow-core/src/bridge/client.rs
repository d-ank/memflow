use log::{debug, info, trace};

use std::io::{Error, ErrorKind, Result};
use std::net::SocketAddr;
use url::Url;

use tokio::io::AsyncRead;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::runtime::current_thread::Runtime;

#[cfg(any(unix))]
use tokio::net::UnixStream;

use capnp::capability::Promise;
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};

use crate::bridge_capnp::bridge;

use crate::address::{Address, Length};
use crate::arch::Architecture;
use crate::mem::{PhysicalRead, PhysicalWrite, VirtualRead, VirtualWrite};

pub struct BridgeClient {
    bridge: bridge::Client,
    runtime: Runtime,
}

#[cfg(any(unix))]
fn connect_unix(path: &str, opts: Vec<&str>) -> Result<BridgeClient> {
    info!("trying to connect via unix -> {}", path);

    let mut runtime = Runtime::new().unwrap();
    let stream = runtime.block_on(UnixStream::connect(path))?;
    let (reader, writer) = stream.split();

    info!("unix connection established -> {}", path);

    Ok(BridgeClient {
        bridge: connect_rpc(&mut runtime, reader, writer)?,
        runtime: runtime,
    })
}

#[cfg(not(any(unix)))]
fn connect_unix(path: &str, opts: Vec<&str>) -> Result<BridgeClient> {
    Err(Error::new(
        ErrorKind::Other,
        "unix sockets are not supported on this os",
    ))
}

fn connect_tcp(path: &str, opts: Vec<&str>) -> Result<BridgeClient> {
    info!("trying to connect via tcp -> {}", path);

    let addr = path
        .parse::<SocketAddr>()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    let mut runtime = Runtime::new().unwrap();
    let stream = runtime.block_on(TcpStream::connect(&addr))?;

    info!("tcp connection established -> {}", path);

    if let Some(_) = opts.iter().filter(|&&o| o == "nodelay").nth(0) {
        info!("trying to set TCP_NODELAY on socket");
        stream.set_nodelay(true).unwrap();
    }

    let (reader, writer) = stream.split();

    Ok(BridgeClient {
        bridge: connect_rpc(&mut runtime, reader, writer)?,
        runtime: runtime,
    })
}

fn connect_rpc<T, U>(runtime: &mut Runtime, reader: T, writer: U) -> Result<bridge::Client>
where
    T: ::std::io::Read + 'static,
    U: ::std::io::Write + 'static,
{
    let network = Box::new(twoparty::VatNetwork::new(
        reader,
        std::io::BufWriter::new(writer),
        rpc_twoparty_capnp::Side::Client,
        Default::default(),
    ));

    let mut rpc_system = RpcSystem::new(network, None);
    let bridge: bridge::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

    runtime.spawn(rpc_system.map_err(|_e| ()));

    Ok(bridge)
}

impl BridgeClient {
    pub fn connect(urlstr: &str) -> Result<BridgeClient> {
        let url = Url::parse(urlstr).map_err(|e| Error::new(ErrorKind::Other, e))?;

        let path = url
            .path()
            .split(",")
            .nth(0)
            .ok_or_else(|| Error::new(ErrorKind::Other, "invalid url"))?;
        let opts = url.path().split(",").skip(1).collect::<Vec<_>>();

        match url.scheme() {
            "unix" => connect_unix(path, opts),
            "tcp" => connect_tcp(path, opts),
            _ => Err(Error::new(ErrorKind::Other, "invalid url scheme")),
        }
    }

    pub fn read_registers(&mut self) -> Result<Vec<u8>> {
        let request = self.bridge.read_registers_request();
        self.runtime
            .block_on(request.send().promise.and_then(|_r| Promise::ok(())))
            .map_err(|_e| Error::new(ErrorKind::Other, "unable to read registers"))
            .and_then(|_v| Ok(Vec::new()))
    }
}

impl PhysicalRead for BridgeClient {
    // physRead @0 (address :UInt64, length :UInt64) -> (data :Data);
    fn phys_read(&mut self, addr: Address, len: Length) -> Result<Vec<u8>> {
        trace!("phys_read({:?}, {:?})", addr, len);

        let mut request = self.bridge.phys_read_request();
        request.get().set_address(addr.as_u64());
        request.get().set_length(len.as_u64());
        self.runtime
            .block_on(
                request.send().promise.and_then(|response| {
                    Promise::ok(Vec::from(pry!(pry!(response.get()).get_data())))
                }),
            )
            .map_err(|_e| Error::new(ErrorKind::Other, "unable to read memory"))
            .and_then(|v| Ok(v))
    }
}

impl PhysicalWrite for BridgeClient {
    // physWrite @1 (address :UInt64, data: Data) -> (length :UInt64);
    fn phys_write(&mut self, addr: Address, data: &Vec<u8>) -> Result<Length> {
        trace!("phys_write({:?})", addr);

        let mut request = self.bridge.phys_write_request();
        request.get().set_address(addr.as_u64());
        request.get().set_data(data);
        self.runtime
            .block_on(
                request.send().promise.and_then(|response| {
                    Promise::ok(Length::from(pry!(response.get()).get_length()))
                }),
            )
            .map_err(|_e| Error::new(ErrorKind::Other, "unable to write memory"))
            .and_then(|v| Ok(v))
    }
}

impl BridgeClient {
    // virtRead @2 (arch: UInt8, dtb :UInt64, address :UInt64, length :UInt64) -> (data: Data);
    fn virt_read_chunk(
        &mut self,
        arch: Architecture,
        dtb: Address,
        addr: Address,
        len: Length,
    ) -> Result<Vec<u8>> {
        let mut request = self.bridge.virt_read_request();
        request.get().set_arch(arch.instruction_set.as_u8());
        request.get().set_dtb(dtb.as_u64());
        request.get().set_address(addr.as_u64());
        request.get().set_length(len.as_u64());
        self.runtime
            .block_on(
                request.send().promise.and_then(|response| {
                    Promise::ok(Vec::from(pry!(pry!(response.get()).get_data())))
                }),
            )
            .map_err(|_e| Error::new(ErrorKind::Other, "unable to read memory"))
            .and_then(|v| Ok(v))
    }

    // virtWrite @3 (arch: UInt8, dtb: UInt64, address :UInt64, data: Data) -> (length :UInt64);
    fn virt_write_chunk(
        &mut self,
        arch: Architecture,
        dtb: Address,
        addr: Address,
        data: &Vec<u8>,
    ) -> Result<Length> {
        let mut request = self.bridge.virt_write_request();
        request.get().set_arch(arch.instruction_set.as_u8());
        request.get().set_dtb(dtb.as_u64());
        request.get().set_address(addr.as_u64());
        request.get().set_data(data);
        self.runtime
            .block_on(
                request.send().promise.and_then(|response| {
                    Promise::ok(Length::from(pry!(response.get()).get_length()))
                }),
            )
            .map_err(|_e| Error::new(ErrorKind::Other, "unable to write memory"))
            .and_then(|v| Ok(v))
    }
}

//
// TODO: split up sections greater than 32mb into multiple packets due to capnp limitations!
//
impl VirtualRead for BridgeClient {
    fn virt_read(
        &mut self,
        arch: Architecture,
        dtb: Address,
        addr: Address,
        len: Length,
    ) -> Result<Vec<u8>> {
        trace!("virt_read({:?}, {:?}, {:?}, {:?})", arch, dtb, addr, len);

        if len > Length::from_mb(32) {
            info!("virt_read(): reading multiple 32mb chunks");
            let mut result: Vec<u8> = vec![0; len.as_usize()];

            let mut base = addr;
            let end = addr + len;
            while base < end {
                let mut clamped_len = Length::from_mb(32);
                if base + clamped_len > end {
                    clamped_len = end - base;
                }

                info!("virt_read(): reading chunk at {:x}", base);
                let mem = self.virt_read_chunk(arch, dtb, base, clamped_len)?;
                let start = (base - addr).as_usize();
                mem.iter().enumerate().for_each(|(i, b)| {
                    result[start + i] = *b;
                });

                base += clamped_len;
            }

            Ok(result)
        } else {
            self.virt_read_chunk(arch, dtb, addr, len)
        }
    }
}

impl VirtualWrite for BridgeClient {
    fn virt_write(
        &mut self,
        arch: Architecture,
        dtb: Address,
        addr: Address,
        data: &Vec<u8>,
    ) -> Result<Length> {
        // TODO: implement chunk logic
        trace!("virt_write({:?}, {:?}, {:?})", arch, dtb, addr);
        self.virt_write_chunk(arch, dtb, addr, data)
    }
}
