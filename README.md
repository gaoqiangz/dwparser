# dwparser
[![github](https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github)](https://github.com/gaoqiangz/dwparser) <br>
[![crates.io](https://meritbadge.herokuapp.com/dwparser)](https://crates.io/crates/dwparser)
[![docs.rs](https://docs.rs/dwparser/badge.svg)](https://docs.rs/dwparser)
![BSD-2-Clause licensed](https://img.shields.io/crates/l/dwparser.svg)

DataWindow Syntax Parser written in Rust

# 功能

- 解析`DataWindow`语法生成语法树`AST`，修改`AST`并重新生成`DataWindow`语法字符串
- 兼容`DataWindow::Modify/Describe`函数的语法，并且可以修改任何语法项

# Feature flags

| Flag              | Description                                              | Default    |
|-------------------|----------------------------------------------------------|------------|
| `preserve_order` | 保留原始语法项的顺序                                              | `enabled`  |
| `case_insensitive` | 忽略大小写                                            | `false`  |
| `query`    | 支持`modify`和`describe`操作                                              | `false`  |
| `serde_support`         | 支持`serde`序列化接口                      | `false`  |
| `full`         | 开启所有特性                      | `false`  |

# 环境要求

- rustc: 最低1.54 **(支持stable)**

# 开始使用

- `cargo add`

```bash
> cargo add dwparser
```

- 或手动添加依赖到`Cargo.toml`

```toml
[dependencies]
dwparser = "0.1.0"
```

# 示例

- 修改`processing`

```rust
let mut dwsyn = dwparser::parse("{DataWindow Syntax}").unwrap();
dwsyn.modify("DataWindow.Processing = 2");
```

- 修改字段类型

```rust
let mut dwsyn = dwparser::parse("{DataWindow Syntax}").unwrap();
//通过字段名
dwsyn.modify("DataWindow.Table.Column.ColA.ColType = 'char(255)'");
//通过ID
dwsyn.modify("DataWindow.Table.Column.1.ColType = 'char(255)'");
```

- 删除字段

```rust
let mut dwsyn = dwparser::parse("{DataWindow Syntax}").unwrap();
//通过字段名
dwsyn.modify("Destroy DataWindow.Table.Column.ColA");
//通过ID
dwsyn.modify("Destroy DataWindow.Table.Column.1");
```

- 删除分组

```rust
let mut dwsyn = dwparser::parse("{DataWindow Syntax}").unwrap();
dwsyn.modify("Destroy DataWindow.Footer.1");
```
