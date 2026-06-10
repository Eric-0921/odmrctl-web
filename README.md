# odmrctl-web-rebuild

这是一个从头开始的最小重建工作树。

当前只包含两类内容：

- `docs/equipment_manual/`：设备手册真值文档与冻结参考资料
- `crates/*-commands`：第一版设备命令 helper

约束：

- 只做命令 helper，不做 transport
- 只做命令 helper，不做 runtime
- helper 名称必须带设备型号前缀
- 注释和文档统一使用中文

