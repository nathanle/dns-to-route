// SPDX-License-Identifier: MIT

use std::{env, net::Ipv4Addr, net::IpAddr};
use hickory_resolver::{Resolver, lookup_ip::LookupIp};

use futures_util::TryStreamExt;
use ipnetwork::Ipv4Network;
use rtnetlink::{new_connection, Error, Handle, RouteMessageBuilder};
use netlink_packet_route::route::{RouteAddress, RouteAttribute, RouteMessage, RouteProtocol};


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
    let current_routes = RouteMessageBuilder::<Ipv4Addr>::new()
        .build();
    let routes_stream = handle.route().get(current_routes).execute();
    let routes = routes_stream.try_collect::<Vec<_>>().await.unwrap();
    //while let Ok(Some(route)) = routes_stream.try_next().await {
        //let test = route.attributes
        //    .iter()
        //    .find(|item| {
        //        matches!(item, RouteAttribute::Destination(RouteAddress::Inet(_)))
        //    });
        //println!("{:?}", test);

    for x in &routes {
        if matches_protocol(&x, RouteProtocol::Babel) {
            if process_route(handle.clone(), x.clone(), response.clone()).await {

            }
        }
    }
    for a in response.iter() {
        if a.is_ipv4() {
            let dest: Ipv4Network = format!("{}/32", a).parse().unwrap_or_else(|_| {
                eprintln!("invalid destination");
                std::process::exit(1);
            });
            println!("{}", a);
            if let Err(e) = add_route(&dest, iface_idx, source, handle.clone(), a).await {
                eprintln!("{e}");
            };
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

    println!("Adding route: {}", address);
    let route = RouteMessageBuilder::<Ipv4Addr>::new()
        .destination_prefix(dest.ip(), dest.prefix())
        .output_interface(iface_idx)
        .protocol(DNS_ROUTE)
        .pref_source(source)
        .build();
    handle.route().add(route).execute().await?;
    Ok(())
}

fn matches_protocol(route: &RouteMessage, target: RouteProtocol) -> bool {
    if route.header.protocol == target {
        return true;
    }
    false
}

async fn process_route(
    handle: Handle,
    route: RouteMessage,
    response: LookupIp
) -> bool {
    for attribute in route.attributes.iter() {
        if let RouteAttribute::Destination(route_addr) = attribute {
            let dest_ip: IpAddr = match route_addr {
                RouteAddress::Inet(ipv4) => IpAddr::V4(*ipv4),
                _ => continue,
            };
            if response.iter().any(|x| x == dest_ip) {
                println!("Route for {} exists. No modifications needed.", dest_ip);
                return false 
            } else {
                println!("I am deleting the route for {:?} as it is not in DNS results", dest_ip);
                let _ = handle
                    .route()
                    .del(route)
                    .execute()
                    .await
                    .map_err(|e| format!("Failed to delete route: {}", e));
                return false
            }
            
        }
    }
    return true 
}


fn usage() {
    eprintln!(
        "usage:
        dns-to-route <DNS record to resolve> <interface> <source>"
    );
}
