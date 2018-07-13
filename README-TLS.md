

```
openssl req -new -newkey rsa:4096 -x509 -sha256 -days 365 -nodes -out Samotop.crt -keyout Samotop.key
```

```
openssl pkcs12 -export -out Samotop.pfx -inkey Samotop.key -in Samotop.crt
```

```
openssl s_client -connect localhost:12345
```

```
openssl s_client -connect localhost:12345 -starttls smtp
```