# BimCraft for Windows

**Available from 0.4.** Download `BimCraft-0.4-setup.exe` from [Releases](https://github.com/getcadcraft/bimcraft/releases).

The installer is the [browser edition](../web/), unchanged, in a WebView2 window (part of Windows 10 and 11), built by [getcadcraft/desktop-shell](https://github.com/getcadcraft/desktop-shell) (`node build.mjs bimcraft --edition <zip>`). Nothing here to compile.

**Not code-signed yet:** Windows SmartScreen shows "Windows protected your PC"; choose **More info › Run anyway**, and check the file against SHA256SUMS first. The Microsoft Store package follows once it is signed.

## File types

- Opens by default unless another program already does: `.ifc` (IFC model)
- Listed under "Open with": `.rvt` (RVT model), `.rfa` (RFA family), `.rte` (RTE template), `.rft` (RFT family template), `.dwg` (DWG drawing), `.dxf` (DXF drawing), `.bcf` (BCF issues), `.bcfzip` (BCF issues)
