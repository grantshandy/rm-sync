rm_ip := "10.120.46.114"

build:
    nix build

send: build
    scp result/bin/rm-cloudshim root@{{ rm_ip }}:/home/root/

login:
    ssh root@{{ rm_ip }}