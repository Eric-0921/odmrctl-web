# odmr-config

**Layer 2** — Canonical JSON configuration for ODMR stations and runtime.

## 职责

- Canonical JSON 配置入口：`AppConfig`、`StationConfig`、`StationDeviceConfig`
- 设备默认值：`Smb100aDefaults`、`Oe1022dDefaults`、`Oe1022dPllReferenceDefaults`、`PllReferenceContract`
- 电源默认值：`MaynuoM8812Defaults`、`CniLaserDefaults`
- 运行期有效默认值：`EffectiveRuntimeDefaults`
- 安全与清理策略：`StationSafetyConfig`、`StationCleanupPolicy`、`ArtifactPolicy`
- 回放与特性开关：`ReplayDefaults`、`FeatureFlags`
- Legacy station profile JSON 兼容加载（字段自动归一化）
- 手册默认值固化来源：SMB100A、OE1022D、Maynuo M8812、CNI Laser 共 4 份设备手册

## 依赖

- `odmr-types`
- `serde`
- `serde_json`

## 参考

- `docs/decisions/config-compatibility-mapping.md` — canonical ↔ legacy 字段映射表
