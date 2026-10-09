pub const OVERVIEW: &str = r#"Dion — 文件与文件夹备注

用途：查看、列出、设置和删除 descript.ion 中的备注。

用法：
  dion [--json] get <path>
  dion [--json] list [directory] [--long|-l|--tree] [--recursive|-r]
  dion [--json] set <path> (<comment> | --stdin | --comment-file <file> | --edit)
  dion [--json] remove <path>
  dion help [command]

命令：
  get      查看条目自身的备注，从父目录的 descript.ion 读取
  list     查询目录内部条目的备注；directory 默认当前目录
  set      设置条目自身的备注；非空正文要求目标存在
  remove   删除条目自身的备注；无记录时成功且不修改文件

参数与默认行为：
  --json      输出 JSON；默认输出文本，set/remove 成功时静默
  --help, -h  显示中文帮助；也可使用 help <command>
  --          结束选项解析，以 - 开头的路径或正文放在其后
  list 默认双栏、只查询当前层；增强选项详见 dion help list
  set 四种正文来源互斥；编辑器配置与清空规则详见 dion help set
  stdin、正文文件和编辑文本使用 UTF-8，接受可选 BOM；备注文件须带 UTF-8 BOM
  帮助写 stdout，退出码 0；带 --json 仍显示文本，不查询或修改备注

常用示例（PowerShell，非空设置要求目标已存在）：
  dion set '.\报告.txt' '项目验收报告'
  dion get '.\资料'                 # 资料条目自身的备注
  dion list '.\资料'                # 资料目录内部的备注
  dion list --tree
  dion set '.\报告.txt' --edit
  dion get '.\报告.txt' --json
  dion remove '.\报告.txt'
  dion set -- '-报告.txt' '-正文'
  dion help set
"#;

/// Returns the Chinese help for an existing command without performing an operation.
pub fn command(name: &str) -> Option<&'static str> {
    match name {
        "get" => Some(GET),
        "list" => Some(LIST),
        "set" => Some(SET),
        "remove" => Some(REMOVE),
        _ => None,
    }
}

const GET: &str = r#"用途：查看文件或文件夹条目自身的备注。

用法：dion [--json] get <path>

参数与默认行为：
  <path>      必填条目路径；从父目录的 descript.ion 读取，无需目标存在
  --json      输出 name、comment、extension；默认只输出正文，不额外添加换行
  --help, -h  显示本帮助，也可使用 dion help get；不需要 path
  --          结束选项解析，以 - 开头的路径放在其后
  未找到备注时退出码 3；已有空记录成功且输出为空
  get '.\资料' 查询资料自身；list '.\资料' 查询目录内部的备注
  备注文件须为带 BOM 的 UTF-8；帮助即使带 --json 也输出文本

常用示例（PowerShell）：
  dion get '.\报告.txt'
  dion get '.\资料'
  dion get '.\报告.txt' --json
  dion get -- '-报告.txt'
"#;

const LIST: &str = r#"用途：列出目录内部条目的备注，包括孤立记录；不修改备注文件。

用法：dion [--json] list [directory] [--long|-l|--tree] [--recursive|-r]

参数与默认行为：
  [directory]      可选起始目录，默认当前目录；查询内部 descript.ion 的记录
  --long, -l       单栏：名称独占一行，正文缩进四个空格，记录间留空行
  --recursive, -r  普通递归：先输出当前层全部记录，再按子目录自然顺序深度优先遍历
  --tree           树状展示，自动递归；与 --long（-l）互斥，带 --json 时也校验
  --json           输出 entries 数组，每条含 name、comment、extension
  --help, -h       显示本帮助，也可使用 dion help list
  --               结束选项解析，以 - 开头的目录路径放在其后
  默认双栏（名称与正文相隔两个空格），只查询当前层，不主动折行或截断正文
  目录优先，各组按 Windows 不区分大小写自然排序，如 file2 在 file10 前
  文本目录名末尾加反斜杠；递归名称为相对于起始目录的路径，JSON 不加目录标记
  递归途中不进入目录符号链接或 Junction，但列出链接自身备注；显式链接起始目录读取内部
  树只显示备注及必要的辅助目录节点，不显示起始根节点；目录备注与展开节点合并
  树 JSON 为普通递归 JSON，不含辅助目录节点；排版选择不改变 JSON 字段
  递归遇到访问、编码或格式错误时整次失败，stdout 为空
  list '.\资料' 查询目录内部；get '.\资料' 查询条目自身的备注
  备注文件须为带 BOM 的 UTF-8；帮助即使带 --json 也输出文本

常用示例（PowerShell）：
  dion list
  dion list '.\资料' -l
  dion list '.\资料' -r --long
  dion list '.\资料' --tree
  dion list '.\资料' --tree --json
  dion list -- '-资料'
"#;

const SET: &str = r#"用途：设置或覆盖文件、文件夹条目自身的备注，保存在父目录的 descript.ion。

用法：
  dion [--json] set <path> <comment>
  dion [--json] set <path> --stdin
  dion [--json] set <path> --comment-file <file>
  dion [--json] set <path> --edit

参数与默认行为：
  <path>                 必填条目路径；设置非空正文要求目标存在
  <comment>              正文参数；字面量 \n 不解释为换行
  --stdin                从标准输入读取正文
  --comment-file <file>  从 UTF-8 文本文件读取；以 - 开头的文件名加 ./ 前缀
  --edit                 使用外部编辑器，预填已有逻辑正文或空文本
  --json                 输出 changed 布尔值；默认成功时静默
  --help, -h             显示本帮助，也可使用 dion help set；不需要 path 或正文
  --                     结束选项解析，以 - 开头的路径或正文放在其后
  四种输入来源互斥，必须且只能选择一种；正文参数、stdin、正文文件拒绝空字符串和纯空白
  stdin、正文文件及编辑文本严格使用 UTF-8，接受可选 BOM；备注文件须带 BOM
  CRLF、CR 统一为逻辑 LF，不裁剪首尾空白、空行或末尾换行；拒绝 NUL 和控制字符 04
  首次设置创建备注文件；有效正文未变化时不写入；未知程序扩展的非空设置拒绝
  set '.\资料' 操作资料条目自身的备注，不是目录内部的记录

外部编辑默认行为：
  编辑器按非空 VISUAL、非空 EDITOR、notepad.exe（Windows 记事本）的顺序选择
  配置支持带引号的程序路径和参数；必须等待编辑完成并正常退出后读取
  需要等待参数的编辑器请自行配置，例如 VISUAL='code --wait'
  --edit 清空为空字符串时删除备注；已有空记录原样关闭也删除，无记录成功且不写文件
  仅含空格、Tab 或换行的纯空白拒绝，不会删除；其他输入来源仍拒绝空白正文
  孤立记录或未知扩展可清空删除；非空编辑仍校验目标存在性及扩展
  编辑期间备注文件内容或存在性变化报 content_changed，不覆盖其他修改
  启动、编辑、输入校验或保存失败保留已创建的编辑文本，在 stderr 报告恢复路径
  成功清理编辑文本；保存阶段失败可能部分完成，保留恢复文本不保证回滚
  帮助即使带 --json 也输出文本，不启动编辑器

常用示例（PowerShell，非空设置要求目标已存在）：
  dion set '.\报告.txt' '项目验收报告'
  '项目说明' | dion set '.\报告.txt' --stdin  # 请确保管道编码为 UTF-8
  dion set '.\报告.txt' --comment-file '.\备注.txt'
  $env:VISUAL = 'code --wait'
  dion set '.\报告.txt' --edit
  dion set -- '-报告.txt' '-正文'
"#;

const REMOVE: &str = r#"用途：删除文件或文件夹条目自身的备注，保留目标条目。

用法：dion [--json] remove <path>

参数与默认行为：
  <path>      必填条目路径；操作父目录的 descript.ion，无需目标存在
  --json      输出 changed 布尔值；默认成功时静默
  --help, -h  显示本帮助，也可使用 dion help remove；不需要 path
  --          结束选项解析，以 - 开头的路径放在其后
  无记录时成功且不修改文件；删除最后一条记录时删除备注文件
  可删除孤立记录及未知程序扩展记录；不删除条目或目录内部的备注文件
  备注文件须为带 BOM 的 UTF-8；帮助即使带 --json 也输出文本

常用示例（PowerShell）：
  dion remove '.\报告.txt'
  dion remove '.\资料'
  dion remove '.\报告.txt' --json
  dion remove -- '-报告.txt'
"#;
