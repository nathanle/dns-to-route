// SPDX-License-Identifier: MIT

use std::{env, net::Ipv4Addr, net::IpAddr};
use hickory_resolver::{Resolver};

use futures_util::TryStreamExt;
use ipnetwork::Ipv4Network;
use rtnetlink::{new_connection, Error, Handle, RouteMessageBuilder};
use netlink_packet_route::route::{RouteAddress, RouteAttribute, RouteProtocol};

const DNS_ROUTE: RouteProtocol = RouteProtocol::Other(42); 

#[tokio::main]
async fn main() -> Result<(), ()> {
    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        println!("Argument count: {}", args.len());
        usage();
        return Ok(());
    }

    let dnsname: String = args[1].parse().unwrap_or_else(|_| {
        eprintln!("invalid DNS name");
        std::process::exit(1);
    });

    let iface: String = args[2].parse().unwrap_or_else(|_| {
        eprintln!("invalid interface");
        std::process::exit(1);
    });
    let iface = iface.into();
    let iface_idx = handle
        .link()
        .get()
        .match_name(iface)
        .execute()
        .try_next()
        .await
        .unwrap()
        .unwrap()
        .header
        .index;

    let source: Ipv4Addr = args[3].parse().unwrap_or_else(|_| {
        eprintln!("invalid source");
        std::process::exit(1);
    });

    let resolver = Resolver::builder_tokio().unwrap().build().unwrap();
    let response = resolver.lookup_ip(dnsname).await.unwrap();
    let mut run_once = true;
    let current_routes = RouteMessageBuilder::<Ipv4Addr>::new()
        .build();

    for address in response.iter() {
        if address.is_ipv4() {
            let mut ipv4address: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
            let mut routes = handle.route().get(current_routes.clone()).execute();
            if let IpAddr::V4(v4_addr) = address {
                ipv4address = v4_addr;
            }
            println!("Checking DNS results against current table.");
            let ip = RouteAttribute::Destination(RouteAddress::Inet(ipv4address));
            let mut route_found: bool = false;
            while let Ok(Some(route)) = routes.try_next().await {
                let mut dns_proto_exists = false;
                let protocol = route.header.protocol; 
                if protocol == RouteProtocol::Babel {
                    dns_proto_exists = true;
                }
                for r in &route.attributes {
                    let mut raddress: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);
                    if let RouteAttribute::Destination(RouteAddress::Inet(x)) = r {
                        raddress = *x;
                    }
                    let in_dns_results = response.iter().any(|item| match item {
                        IpAddr::V6(_) => false,
                        IpAddr::V4(item) => item == raddress,
                    });
                    if let RouteAttribute::Destination(RouteAddress::Inet(_)) = r && *r == ip && dns_proto_exists {
                        println!("Route exists for {:#?}", ipv4address);
                        route_found = true;
                    }
                    if let RouteAttribute::Destination(RouteAddress::Inet(_)) = r && *r != ip && dns_proto_exists && run_once && !in_dns_results {
                        println!("Route exists for {:#?}, but no longer in DNS.", raddress);
                        if let Err(e) = handle.route().del(route.clone()).execute().await {
                            eprintln!("{e}");
                        }
                        else {
                            println!("Route for {:#?} deleted.", raddress);
                        }
                    }
                }
            }
            if !route_found {
                println!("No matching result exists. Adding DNS result {} to the table.", ipv4address);
                let dest: Ipv4Network = format!("{}/32", address).parse().unwrap_or_else(|_| {
                    eprintln!("invalid destination");
                    std::process::exit(1);
                });
                if let Err(e) = add_route(&dest, iface_idx, source, handle.clone(), address).await {
                    eprintln!("{e}");
                };
            }
        run_once = false;
        }
    }
    Ok(())
}

async fn add_route(
    dest: &Ipv4Network,
    iface_idx: u32,
    source: Ipv4Addr,
    handle: Handle,
    address: IpAddr,
) -> Result<(), Error> {

    let route = RouteMessageBuilder::<Ipv4Addr>::new()
        .destination_prefix(dest.ip(), dest.prefix())
        .output_interface(iface_idx)
        .protocol(DNS_ROUTE)
        .pref_source(source)
        .build();
    handle.route().add(route).execute().await?;
    println!("Route for {} added.", address);
    Ok(())
}

fn usage() {
    eprintln!(
        "usage:
        dns-to-route <DNS record to resolve> <interface> <source>"
    );
}
