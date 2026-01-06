运行在内核中加载并运行用户应用

os:
```shell
cd os
cargo build --release
./objcopy.sh
./run.sh
```

user:
```shell
cd user
./build.sh
./run.sh
```
