> tyx fork note:
>
> 这个 fork 主要用于在无 sudo 的开发机上更方便地使用 Bear，并解决在 preload 模式下 `compile_commands.json` 的编译器可执行文件路径无法稳定输出为绝对路径的问题（例如激活 conda 环境后，系统里可能存在多个 `g++`）。
>
> - 新增 `format.entries.compiler_executable`，用于控制写入条目 `arguments[0]` 的编译器字段表现形式：
>   - `file-name`（上游默认）：只写 basename（例如 `g++`），更便携但不够稳定。
>   - `full-path`：写入 Bear 捕获到的可执行文件路径原值；若拦截上报本身就是绝对路径（例如 `execve("/usr/bin/g++", ...)` 或 wrapper 模式上报的真实路径），则会是绝对路径；但若上报的是短名（`execvp("g++", ...)`），依旧会是 `g++`。
>   - `resolved-path`：在 `full-path` 基础上，若捕获到的是短名（例如 `g++`），则使用当次执行捕获到的 `PATH` 解析为绝对路径（例如 conda 激活后解析为 `.../envs/cpp_dev/bin/g++`）；解析失败则回退到原值。
> - 本 fork 的 `bear/build.rs` 将默认安装前缀指向 `~/.local/bear`（适配无 sudo 环境）；如在其他机器复用，请按需调整该路径或改为打包/环境变量方案。
>
> 最小配置示例（Linux + preload）：
>
> ```yaml
> schema: "4.0"
> intercept:
>   mode: preload
>   path: /home/tyx/.local/bear/libexec/bear/lib/x86_64-linux-gnu/libexec.so
> format:
>   entries:
>     compiler_executable: resolved-path
> ```

[![Packaging status](https://repology.org/badge/tiny-repos/bear-clang.svg)](https://repology.org/project/bear-clang/versions)
[![GitHub release](https://img.shields.io/github/release/rizsotto/Bear)](https://github.com/rizsotto/Bear/releases)
[![GitHub Release Date](https://img.shields.io/github/release-date/rizsotto/Bear)](https://github.com/rizsotto/Bear/releases)
[![Continuous Integration](https://github.com/rizsotto/Bear/workflows/rust%20CI/badge.svg)](https://github.com/rizsotto/Bear/actions)
[![Contributors](https://img.shields.io/github/contributors/rizsotto/Bear)](https://github.com/rizsotto/Bear/graphs/contributors)
[![Gitter](https://img.shields.io/gitter/room/rizsotto/Bear)](https://gitter.im/rizsotto/Bear)

ʕ·ᴥ·ʔ Build EAR
===============

Bear is a tool that generates a compilation database for clang tooling.

The [JSON compilation database][JSONCDB] is used in the clang project to
provide information on how a single compilation unit is processed. With this,
it is easy to re-run the compilation with alternate programs.

Some build systems natively support the generation of a JSON compilation
database. For projects that do not use such build tools, Bear generates the
JSON file during the build process.

  [JSONCDB]: http://clang.llvm.org/docs/JSONCompilationDatabase.html

How to install
--------------

Bear is [packaged](https://repology.org/project/bear-clang/versions) for many
distributions. Check your distribution's package manager. Alternatively, you
can [build it](INSTALL.md) from source.

How to use
----------

After installation, use it like this:

    bear -- <your-build-command>

The output file, `compile_commands.json`, is saved in the current directory.

For more options, you can check the man page or pass the `--help` parameter.
Note that if you want to pass parameters to Bear, pass them _before_ the `--`;
everything after that is considered part of the build command.

Please be aware that some package managers still ship the 2.4.x release. In
that case, please omit the extra `--` or consult your local documentation.

For more information, read the man pages or the project [wiki][WIKI], which
talks about limitations, known issues, and platform-specific usage.

Problem reports
---------------

Before opening a new problem report, please check the [wiki][WIKI] to see if
your problem is a known issue with a documented workaround. It's also helpful
to look at older (possibly closed) [issues][ISSUES] before opening a new one.

If you decide to report a problem, please provide as much context as possible
to help reproduce the error. If you just have a question about usage, please
don't be shy; ask your question in an issue or in our [chat][CHAT].

If you've found a bug and have a fix for it, please share it by opening a pull
request.

Please follow the [contribution guide][GUIDE] when you do.

  [ISSUES]: https://github.com/rizsotto/Bear/issues
  [WIKI]: https://github.com/rizsotto/Bear/wiki
  [CHAT]: https://gitter.im/rizsotto/Bear/discussions
  [GUIDE]: https://github.com/rizsotto/Bear/blob/master/CONTRIBUTING.md
