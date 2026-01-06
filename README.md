点亮CPU

```shell
cargo build --release
./objcopy.sh
# 启动内核并打开 1234 端口等待 gdb 调试器接入
./run.sh
# 打开另一个终端，启动 gdb 连接qemu 发送调试指令
./debug.sh
```
