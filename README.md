# dns-to-route

Command syntax:
```
usage:
        dns-to-route <DNS record to resolve> <interface> <source>
```

Example:
```
#> dns-to-route www.example.com eth0 172.236.110.200
Checking status of address: 104.20.23.154
No matching result exists. Adding DNS result 104.20.23.154 to the table.
Route for 104.20.23.154 added.
Checking status of address: 172.66.147.243
No matching result exists. Adding DNS result 172.66.147.243 to the table.
Route for 172.66.147.243 added.```

We did a DNS request on www.example.com and we recieved two IPv4 addresses back. We are going to add a route for each IP to use the eth0 interface and source from 172.236.110.200

If one or all of the routes exist, we will skipp the step of adding the route:
```
#> ip route del 172.66.147.243 dev eth0 proto babel src 172.236.110.200
#> dns-to-route www.example.com eth0 172.236.110.200
Checking status of address: 172.66.147.243
No matching result exists. Adding DNS result 172.66.147.243 to the table.
Route for 172.66.147.243 added.
Checking status of address: 104.20.23.154
Route exists for 104.20.23.154
```

If DNS no longer reports an IP, we will remove that route:

```
Checking status of address: 104.20.23.154
Route exists for 8.8.8.8, but no longer in DNS.
Route for 8.8.8.8 deleted.
Route exists for 104.20.23.154
Checking status of address: 172.66.147.243
Route exists for 172.66.147.243
```
