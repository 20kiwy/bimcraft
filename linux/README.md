# BimCraft for Linux

**Available from 0.4.** The `.deb` and AppImage are in [Releases](https://github.com/getcadcraft/bimcraft/releases).

Both are the [browser edition](../web/), unchanged, in a WebKitGTK window, built by [getcadcraft/desktop-shell](https://github.com/getcadcraft/desktop-shell).

## Flathub

[`flathub/`](flathub/) is the Flathub manifest (`com.getcadcraft.bimcraft.yml`) with its desktop entry, AppStream metainfo, MIME types and icons. It builds from public sources only: the shell's tag and the release zip. Values marked `OWNER:` are filled in at submission.

```sh
flatpak-builder --user --install build-dir flathub/com.getcadcraft.bimcraft.yml
```

## File types

- Opens by default unless another program already does: `.ifc` (IFC model)
- Listed under "Open with": `.rvt` (RVT model), `.rfa` (RFA family), `.rte` (RTE template), `.rft` (RFT family template), `.dwg` (DWG drawing), `.dxf` (DXF drawing), `.bcf` (BCF issues), `.bcfzip` (BCF issues)
