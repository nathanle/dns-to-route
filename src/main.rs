use std::net::Ipv4Addr;
//absl // or tokio main setup
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, handle, _) = rtnetlink::new_connection()?;
    tokio::spawn(connection);
    let dest_ip: Ipv4Addr = "192.168.10.0".parse()?;
    let gateway_ip: Ipv4Addr = "192.168.1.1".parse()?;
    let ifindex: u32 = 2; // Target interface index (e.g., eth0)

    handle
        .route()
        .add()
        .v4()
        .destination_prefix(dest_ip, 24)
        .gateway(gateway_ip)
        .output_interface(ifindex)
        .execute()
        .await?;

    println!("Route added successfully!");
    Ok(())
}
