// SPDX-License-Identifier: MIT

use std::{env, net::Ipv4Addr};
use hickory_resolver::Resolver;

use futures_util::TryStreamExt;
use ipnetwork::Ipv4Network;
use rtnetlink::{new_connection, Error, Handle, RouteMessageBuilder};

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
    let source: Ipv4Addr = args[3].parse().unwrap_or_else(|_| {
        eprintln!("invalid source");
        std::process::exit(1);
    });
    let resolver = Resolver::builder_tokio().unwrap().build().unwrap();
    let response = resolver.lookup_ip(dnsname).await.unwrap();
    for a in response.iter() {
        if a.is_ipv4() {
            let dest: Ipv4Network = format!("{}/32", a).parse().unwrap_or_else(|_| {
                eprintln!("invalid destination");
                std::process::exit(1);
            });
            println!("{}", a);
            if let Err(e) = add_route(&dest, &iface, source, handle.clone()).await {
                eprintln!("{e}");
            };
        }
    }
    Ok(())
}

async fn add_route(
    dest: &Ipv4Network,
    iface: impl Into<String>,
    source: Ipv4Addr,
    handle: Handle,
) -> Result<(), Error> {
    let iface = iface.into();
    let iface_idx = handle
        .link()
        .get()
        .match_name(iface)
        .execute()
        .try_next()
        .await?
        .unwrap()
        .header
        .index;

    println!("{}", "Adding route!");
    let route = RouteMessageBuilder::<Ipv4Addr>::new()
        .destination_prefix(dest.ip(), dest.prefix())
        .output_interface(iface_idx)
        .pref_source(source)
        .build();
    handle.route().add(route).execute().await?;
    Ok(())
}

fn usage() {
    eprintln!(
        "usage:
        dns-to-route <DNS record to resolve> <interface> <source>"
    );
}
