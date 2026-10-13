<?php
// SPDX-License-Identifier: Apache-2.0
/**
 * BimCraft browser UI (rendered inside app-shell.inc; $app, $__area, $__build and $__manifest are set).
 * Server-rendered: the three panels, the file picker and the empty-state instructions.
 * bimcraft.js fills the panels from the opened file. Files are read in the browser and
 * never leave the visitor's computer; nothing here posts anywhere.
 *
 *   left   open a file or folder · file/folder facts · storeys · element classes
 *   stage  toolbar (view · Edit: undo, redo, copy, paste, More menu · library) + 3D canvas (IFC) | file card (RVT/RFA) | folder report | empty state
 *   right  tabs: Properties of the picked element | Library (library-view.js: sources, cards, place/swap)
 *
 * "Edit in <drafting app>" on the 2D view: only when that app is open in this area and the build it
 * runs here takes a handed-over file (manifest "accepts": ["handoff"]; see handoff.js).
 */
Lang::use($__build); echo Lang::bootTag();   // language files (docs/LANG.md); harmless once app-shell.inc does it
$__edit = Catalog::app('cadcraft');
$__editBuild = $__edit && Catalog::openIn($__edit, $__area['key']) ? AppBuilds::resolve($__edit['slug'], $__area['key']) : null;
$__editUrl = $__editBuild && in_array('handoff', $__editBuild['manifest']['accepts'] ?? [], true) ? $__area['path'] . '/' . $__edit['slug'] : '';
// "Open in <detailing app>" on an IFC model (0.4 1.5): the same rule — open in this area, and its build takes a handoff.
$__detail = Catalog::app('detailcraft');
$__detailBuild = $__detail && Catalog::openIn($__detail, $__area['key']) ? AppBuilds::resolve($__detail['slug'], $__area['key']) : null;
$__detailUrl = $__detailBuild && in_array('handoff', $__detailBuild['manifest']['accepts'] ?? [], true) ? $__area['path'] . '/' . $__detail['slug'] : '';
?>
<main id="main" class="bc" data-bimcraft<?php if ($__editUrl): ?> data-edit-url="<?= e($__editUrl) ?>" data-edit-app="<?= e($__edit['name']) ?>"<?php endif; ?>>
  <aside class="bc-panel bc-left" aria-label="<?= Lang::e('qto.sp.file') ?>">
    <section class="bc-sec">
      <h2 class="bc-h"><?= Lang::e('ui.open.title') ?></h2>
      <label class="bc-drop" for="bc-file" data-drop>
        <i class="bi bi-folder2-open" aria-hidden="true"></i>
        <span class="bc-drop-main"><?= Lang::e('ui.drop.main') ?></span>
        <span class="bc-drop-sub"><?= Lang::e('ui.drop.sub') ?></span>
        <input type="file" id="bc-file" class="visually-hidden" accept=".ifc,.rvt,.rfa,.rte,.rft,.dwg,.dxf,.pln,.pla,.tpl,.bpn" data-file>
      </label>
      <button type="button" class="btn-line bc-folder-btn" data-pick-folder><i class="bi bi-folder-symlink" aria-hidden="true"></i> <?= Lang::e('ui.open.folder') ?></button>
      <input type="file" class="visually-hidden" webkitdirectory multiple tabindex="-1" aria-hidden="true" data-folder-input>
      <input type="file" class="visually-hidden" accept=".dwg,.png,.jpg,.jpeg,.gif,.bmp,.webp,.tif,.tiff" multiple tabindex="-1" aria-hidden="true" data-xref-input>
      <div class="bc-recent" data-recent hidden>
        <p class="bc-recent-h"><?= Lang::e('ui.recent') ?></p>
        <ul></ul>
      </div>
      <?php /* recent files (host.js 1.4): the shell fills it with files it can reopen; empty = hidden */ ?>
      <div data-recent-files data-max="6"></div>
      <p class="bc-note"><i class="bi bi-hand-index" aria-hidden="true"></i> <?= Lang::e('ui.tip.drag') ?></p>
      <p class="bc-note"><i class="bi bi-shield-check" aria-hidden="true"></i> <?= Lang::e('ui.privacy') ?></p>
    </section>
    <section class="bc-sec" data-changes hidden>
      <h2 class="bc-h"><?= Lang::e('ui.changes') ?></h2>
      <ol class="bc-changes" data-change-list></ol>
      <div class="bc-change-actions">
        <button type="button" class="btn-line btn-sm-cc" data-undo disabled title="<?= Lang::e('ui.undo.title') ?>"><i class="bi bi-arrow-counterclockwise" aria-hidden="true"></i> <?= Lang::e('ui.undo') ?></button>
        <button type="button" class="btn-plot btn-sm-cc" data-save disabled title="<?= Lang::e('ui.save.title') ?>"><i class="bi bi-download" aria-hidden="true"></i><span> <?= Lang::e('ui.save') ?></span></button>
      </div>
      <p class="bc-note"><?= Lang::e('ui.save.note') ?></p>
    </section>
    <section class="bc-sec" data-file-info hidden>
      <h2 class="bc-h"><?= Lang::e('qto.sp.file') ?></h2>
      <dl class="bc-dl" data-file-dl></dl>
      <button type="button" class="bc-linkbtn bc-back" data-folder-back hidden><i class="bi bi-arrow-left" aria-hidden="true"></i> <?= Lang::e('ui.folder.back') ?></button>
    </section>
    <section class="bc-sec" data-layers hidden>
      <h2 class="bc-h"><?= Lang::e('ui.layers') ?></h2>
      <p class="bc-layer-all"><button type="button" class="bc-linkbtn" data-layers-all="1"><?= Lang::e('ui.layers.allon') ?></button> <button type="button" class="bc-linkbtn" data-layers-all="0"><?= Lang::e('ui.layers.alloff') ?></button></p>
      <ul class="bc-checks" data-layer-list></ul>
    </section>
    <section class="bc-sec" data-models-sec hidden>
      <h2 class="bc-h"><?= Lang::e('fed.title') ?></h2>
      <p class="bc-note"><?= Lang::e('fed.hint') ?></p>
      <ul class="bc-models" data-model-list></ul>
      <button type="button" class="btn-line btn-sm-cc bc-model-add" data-model-add><i class="bi bi-plus-lg" aria-hidden="true"></i> <?= Lang::e('fed.add') ?></button>
      <?php /* Compare versions (0.4 B21): shown with two models or more; bimcraft.js "Compare versions", ifc-compare.js. */ ?>
      <details class="bc-find-details" data-compare-details hidden>
        <summary class="bc-h"><?= Lang::e('cmp.title') ?></summary>
        <div data-compare></div>
      </details>
      <input type="file" class="visually-hidden" accept=".ifc" multiple tabindex="-1" aria-hidden="true" data-model-input>
    </section>
    <section class="bc-sec" data-find-sec hidden>
      <?php /* Find (0.4 B14): built when opened (it reads every element's properties); bimcraft.js "Find". */ ?>
      <details class="bc-find-details" data-find-details>
        <summary class="bc-h"><?= Lang::e('find.title') ?></summary>
        <div data-find></div>
      </details>
    </section>
    <section class="bc-sec" data-qto-sec hidden>
      <?php /* Quantity totals (0.4 B20): built when opened; bimcraft.js "Quantity totals", ifc-qto.js. */ ?>
      <details class="bc-find-details" data-qto-details>
        <summary class="bc-h"><?= Lang::e('qto.title') ?></summary>
        <div data-qto></div>
      </details>
    </section>
    <section class="bc-sec" data-4d-sec hidden>
      <?php /* 4D schedule (0.4 B22): built when opened; bimcraft.js "4D schedule", ifc-4d.js. */ ?>
      <details class="bc-find-details" data-4d-details>
        <summary class="bc-h"><?= Lang::e('fd.title') ?></summary>
        <input type="file" accept=".csv,.txt,text/csv" hidden data-4d-input>
        <div data-4d></div>
      </details>
    </section>
    <section class="bc-sec" data-checks-sec hidden>
      <?php /* Model checks (0.4 B19): built when opened; bimcraft.js "Model checks", ifc-checks.js. */ ?>
      <details class="bc-find-details" data-checks-details>
        <summary class="bc-h"><?= Lang::e('checks.title') ?></summary>
        <div data-checks></div>
      </details>
    </section>
    <section class="bc-sec" data-clash-sec hidden>
      <?php /* Clash check (0.4 B17): built when opened; bimcraft.js "Clash check", clash.js in clash-worker.js. */ ?>
      <details class="bc-find-details" data-clash-details>
        <summary class="bc-h"><?= Lang::e('clash.title') ?></summary>
        <div data-clash></div>
      </details>
    </section>
    <section class="bc-sec" data-tree-sec hidden>
      <h2 class="bc-h" id="bc-tree-h"><?= Lang::e('tree.title') ?></h2>
      <ul class="bc-tree" data-tree aria-labelledby="bc-tree-h"></ul>
    </section>
    <section class="bc-sec" data-storeys hidden>
      <h2 class="bc-h"><?= Lang::e('ui.storeys') ?></h2>
      <ul class="bc-checks" data-storey-list></ul>
    </section>
    <section class="bc-sec" data-classes hidden>
      <h2 class="bc-h"><?= Lang::e('ui.elements') ?></h2>
      <ul class="bc-counts" data-class-list></ul>
    </section>
  </aside>

  <section class="bc-stage" aria-label="<?= Lang::e('ui.stage.label') ?>">
    <div class="bc-toolbar" role="toolbar" aria-label="<?= Lang::e('ui.toolbar.view') ?>" data-toolbar hidden>
      <button type="button" class="bc-tool" data-view="iso" title="<?= Lang::e('ui.view.fit.title') ?>"><i class="bi bi-box" aria-hidden="true"></i><span><?= Lang::e('ui.view.fit') ?></span></button>
      <button type="button" class="bc-tool" data-view="top" title="<?= Lang::e('ui.view.top.title') ?>"><i class="bi bi-square" aria-hidden="true"></i><span><?= Lang::e('viewcube.face.top') ?></span></button>
      <button type="button" class="bc-tool" data-view="front" title="<?= Lang::e('ui.view.front.title') ?>"><i class="bi bi-layout-sidebar-inset" aria-hidden="true"></i><span><?= Lang::e('viewcube.face.front') ?></span></button>
      <button type="button" class="bc-tool" data-view="side" title="<?= Lang::e('ui.view.side.title') ?>"><i class="bi bi-layout-sidebar-inset-reverse" aria-hidden="true"></i><span><?= Lang::e('ui.view.side') ?></span></button>
      <span class="bc-tool-sep" aria-hidden="true"></span>
      <button type="button" class="bc-tool" data-toggle="edges" aria-pressed="true" title="<?= Lang::e('ui.edges.title') ?>"><i class="bi bi-bounding-box" aria-hidden="true"></i><span><?= Lang::e('ui.edges') ?></span></button>
      <button type="button" class="bc-tool" data-toggle="spaces" aria-pressed="false" title="<?= Lang::e('ui.spaces.title') ?>"><i class="bi bi-grid-3x3" aria-hidden="true"></i><span><?= Lang::e('qto.view.spaces') ?></span></button>
      <span class="bc-tool-sep" aria-hidden="true"></span>
      <?php /* Edit: bimcraft.js (search "Edit menu") keeps aria-disabled and each title (the reason when it cannot act) up to date. */ ?>
      <button type="button" class="bc-tool" data-act="undo" title="<?= Lang::e('ui.undo.title') ?>"><i class="bi bi-arrow-counterclockwise" aria-hidden="true"></i><span><?= Lang::e('ui.undo') ?></span></button>
      <button type="button" class="bc-tool" data-act="redo" title="<?= Lang::e('ui.redo.title') ?>"><i class="bi bi-arrow-clockwise" aria-hidden="true"></i><span><?= Lang::e('ui.redo') ?></span></button>
      <button type="button" class="bc-tool" data-act="copy" title="<?= Lang::e('edit.copy.title') ?>"><i class="bi bi-copy" aria-hidden="true"></i><span><?= Lang::e('ui.copy') ?></span></button>
      <button type="button" class="bc-tool" data-act="paste" title="<?= Lang::e('edit.paste.title') ?>"><i class="bi bi-clipboard-plus" aria-hidden="true"></i><span><?= Lang::e('ui.paste') ?></span></button>
      <details class="bc-menu" data-edit-menu>
        <summary class="bc-tool" title="<?= Lang::e('ui.more.title') ?>"><i class="bi bi-three-dots" aria-hidden="true"></i><span><?= Lang::e('ui.more') ?></span></summary>
        <div class="bc-menu-list" role="menu" aria-label="<?= Lang::e('ui.more.label') ?>">
          <button type="button" role="menuitem" data-act="cut"><?= Lang::e('ui.cut') ?> <kbd>Ctrl+X</kbd></button>
          <button type="button" role="menuitem" data-act="copy"><?= Lang::e('ui.copy') ?> <kbd>Ctrl+C</kbd></button>
          <button type="button" role="menuitem" data-act="paste"><?= Lang::e('ui.paste') ?> <kbd>Ctrl+V</kbd></button>
          <button type="button" role="menuitem" data-act="paste-in-place"><?= Lang::e('ui.paste.inplace') ?> <kbd>Ctrl+Shift+V</kbd></button>
          <button type="button" role="menuitem" data-act="duplicate"><?= Lang::e('ui.duplicate') ?> <kbd>Ctrl+D</kbd></button>
          <button type="button" role="menuitem" data-act="delete"><?= Lang::e('ui.delete') ?> <kbd>Delete</kbd></button>
          <hr>
          <button type="button" role="menuitem" data-act="select-all"><?= Lang::e('ui.selectall') ?> <kbd>Ctrl+A</kbd></button>
          <button type="button" role="menuitem" data-act="select-none"><?= Lang::e('ui.selectnone') ?> <kbd>Esc</kbd></button>
        </div>
      </details>
      <span class="bc-tool-sep" aria-hidden="true"></span>
      <button type="button" class="bc-tool" data-lib-tool="library" title="<?= Lang::e('ui.lib.title') ?>"><i class="bi bi-collection" aria-hidden="true"></i><span><?= Lang::e('ui.library') ?></span></button>
      <button type="button" class="bc-tool" data-lib-tool="equipment" aria-pressed="false" title="<?= Lang::e('ui.equip.title') ?>"><i class="bi bi-list-check" aria-hidden="true"></i><span><?= Lang::e('ui.equipment') ?></span></button>
      <button type="button" class="bc-tool" data-lib-tool="updates" title="<?= Lang::e('ui.updates.title') ?>"><i class="bi bi-arrow-repeat" aria-hidden="true"></i><span><?= Lang::e('ui.updates') ?></span></button>
      <button type="button" class="bc-tool" data-lib-tool="credits" title="<?= Lang::e('ui.credits.title') ?>"><i class="bi bi-c-circle" aria-hidden="true"></i><span><?= Lang::e('ui.credits') ?></span></button>
      <span class="bc-tool-sep" aria-hidden="true"></span>
      <?php /* Show (0.4 B12): bimcraft.js (search "Hide and isolate") keeps aria-disabled and the titles up to date. A menu, so the toolbar keeps its two rows. */ ?>
      <details class="bc-menu" data-vis-menu>
        <summary class="bc-tool" title="<?= Lang::e('vis.menu.title') ?>"><i class="bi bi-eye" aria-hidden="true"></i><span><?= Lang::e('vis.toolbar') ?></span></summary>
        <div class="bc-menu-list" role="menu" aria-label="<?= Lang::e('vis.toolbar') ?>">
          <button type="button" role="menuitem" data-vis="hide" title="<?= Lang::e('vis.hide.title') ?>"><?= Lang::e('vis.hide') ?> <kbd>H</kbd></button>
          <button type="button" role="menuitem" data-vis="isolate" title="<?= Lang::e('vis.isolate.title') ?>"><?= Lang::e('vis.isolate') ?> <kbd>I</kbd></button>
          <button type="button" role="menuitem" data-vis="ghost" title="<?= Lang::e('vis.ghost.title') ?>"><?= Lang::e('vis.ghost') ?> <kbd>T</kbd></button>
          <hr>
          <button type="button" role="menuitem" data-vis="all" title="<?= Lang::e('vis.showall.title') ?>"><?= Lang::e('vis.showall') ?> <kbd>Shift+H</kbd></button><?php /* i18n-ignore: key names */ ?>
        </div>
      </details>
      <span class="bc-tool-sep" aria-hidden="true"></span>
      <button type="button" class="bc-tool" data-toggle="section" aria-pressed="false" title="<?= Lang::e('ui.section.title') ?>"><i class="bi bi-scissors" aria-hidden="true"></i><span><?= Lang::e('sec.button') ?></span></button>
      <button type="button" class="bc-tool" data-toggle="walk" aria-pressed="false" title="<?= Lang::e('walk.title') ?>"><i class="bi bi-person-walking" aria-hidden="true"></i><span><?= Lang::e('walk.button') ?></span></button>
      <?php if ($__detailUrl): ?>
      <span class="bc-tool-sep" aria-hidden="true"></span>
      <button type="button" class="bc-tool" data-handoff-ifc data-handoff-url="<?= e($__detailUrl) ?>" data-handoff-app="<?= e($__detail['name']) ?>" title="<?= Lang::e('handoff.ifc.title', ['app' => $__detail['name']]) ?>"><i class="bi bi-box-arrow-up-right" aria-hidden="true"></i><span><?= Lang::e('handoff.ifc.button', ['app' => $__detail['name']]) ?></span></button>
      <?php endif; ?>
      <button type="button" class="bc-tool" data-toggle="measure" aria-pressed="false" title="<?= Lang::e('measure.title') ?>"><i class="bi bi-rulers" aria-hidden="true"></i><span><?= Lang::e('measure.button') ?></span></button>
    </div>
    <?php /* Section plane (viewer3d.js setSection): bimcraft.js (search "Section plane") fills and wires it. */ ?>
    <div class="bc-sectionbar" data-sectionbar hidden>
      <span class="bc-sub"><?= Lang::e('ui.sec.cutfrom') ?></span>
      <button type="button" class="bc-tool" data-sec-axis="top" aria-pressed="true" title="<?= Lang::e('ui.sec.top.title') ?>"><i class="bi bi-square" aria-hidden="true"></i><span><?= Lang::e('viewcube.face.top') ?></span></button>
      <button type="button" class="bc-tool" data-sec-axis="front" aria-pressed="false" title="<?= Lang::e('ui.sec.front.title') ?>"><i class="bi bi-layout-sidebar-inset" aria-hidden="true"></i><span><?= Lang::e('viewcube.face.front') ?></span></button>
      <button type="button" class="bc-tool" data-sec-axis="side" aria-pressed="false" title="<?= Lang::e('ui.sec.side.title') ?>"><i class="bi bi-layout-sidebar-inset-reverse" aria-hidden="true"></i><span><?= Lang::e('ui.view.side') ?></span></button>
      <button type="button" class="bc-tool" data-sec-axis="box" aria-pressed="false" title="<?= Lang::e('sec.box.title') ?>"><i class="bi bi-box" aria-hidden="true"></i><span><?= Lang::e('sec.box') ?></span></button>
      <span class="bc-sec-plane" data-sec-plane>
        <label><?= Lang::e('sec.offset.label') ?> <input type="range" min="0" max="1" step="0.001" value="0.5" data-sec-range aria-describedby="bc-sec-hint"></label>
        <label><span class="visually-hidden"><?= Lang::e('sec.offset.label') ?></span><input type="text" class="bc-input" inputmode="decimal" data-sec-offset aria-describedby="bc-sec-hint"> <span data-sec-unit>m</span></label>
        <span class="bc-sub" id="bc-sec-hint" data-sec-hint><?= Lang::e('ui.sec.hint') ?></span>
        <button type="button" class="bc-tool" data-sec-flip aria-pressed="false" title="<?= Lang::e('ui.sec.flip.title') ?>"><i class="bi bi-arrow-down-up" aria-hidden="true"></i><span><?= Lang::e('ui.sec.flip') ?></span></button>
      </span>
      <?php /* Section box (0.4 B4): from / to per IFC axis, metres from the model's minimum corner; bimcraft.js "Section box". */ ?>
      <span class="bc-sec-box" data-sec-box hidden>
        <?php foreach (['x' => 'sec.box.x', 'y' => 'sec.box.y', 'z' => 'sec.box.z'] as $__ax => $__key): ?>
        <span class="bc-sec-boxrow" data-box-axis="<?= $__ax ?>"><span class="bc-sec-boxlbl"><?= Lang::e($__key) ?></span>
          <input type="range" min="0" max="1" step="0.001" value="0" data-box-from aria-label="<?= Lang::e($__key) ?> <?= Lang::e('sec.box.from') ?>">
          <input type="range" min="0" max="1" step="0.001" value="1" data-box-to aria-label="<?= Lang::e($__key) ?> <?= Lang::e('sec.box.to') ?>">
          <span class="bc-sub" data-box-val></span></span>
        <?php endforeach; ?>
        <button type="button" class="btn-line btn-sm-cc" data-sec-box-pick><?= Lang::e('sec.box.pick') ?></button>
        <button type="button" class="btn-line btn-sm-cc" data-sec-box-all><?= Lang::e('sec.box.all') ?></button>
      </span>
      <button type="button" class="bc-tool" data-sec-fill aria-pressed="true" title="<?= Lang::e('sec.fill.title') ?>"><i class="bi bi-square-fill" aria-hidden="true"></i><span><?= Lang::e('sec.fill') ?></span></button>
      <button type="button" class="btn-line btn-sm-cc" data-sec-off><?= Lang::e('color.off') ?></button>
    </div>
    <div class="bc-placebar" data-placebar hidden>
      <p class="bc-placebar-what"><span data-place-name></span> <span class="bc-sub" data-place-hint><?= Lang::e('ui.place.hint') ?></span></p>
      <div class="bc-placebar-row">
        <label><?= Lang::e('qto.sp.storey') ?> <select class="select-cc" data-place-storey></select></label>
        <label><?= Lang::e('ui.place.xy') ?> <input type="text" class="bc-input" data-place-xy inputmode="decimal" placeholder="1200, 3400" size="14"></label>
        <button type="button" class="btn-line btn-sm-cc" data-place-typed><?= Lang::e('ui.place.here') ?></button>
        <button type="button" class="bc-tool" data-place-snap aria-pressed="false" title="<?= Lang::e('ui.place.snap.title') ?>"><i class="bi bi-magnet" aria-hidden="true"></i><span><?= Lang::e('ui.place.snap') ?></span></button>
        <span class="bc-placebar-xy" data-place-at aria-live="polite"></span>
        <button type="button" class="bc-tool" data-place-turn title="<?= Lang::e('place.turn.title') ?>"><i class="bi bi-arrow-clockwise" aria-hidden="true"></i><span><?= Lang::e('place.turn') ?></span></button>
        <button type="button" class="btn-line btn-sm-cc" data-place-stop><?= Lang::e('ui.place.stop') ?></button>
      </div>
    </div>

    <div class="bc-equip bc-ifc-equip" data-ifc-equip hidden></div>

    <div class="bc-canvas" data-canvas-wrap hidden><canvas data-canvas aria-label="<?= Lang::e('ui.canvas.label') ?>"></canvas><div class="bc-measure-labels" data-measure-labels aria-hidden="true"></div></div>
    <?php /* Measure (0.4 B6): bimcraft.js "Measure" fills and wires it. */ ?>
    <?php /* Walk (0.4 B18): bimcraft.js "Walk" wires it. */ ?>
    <div class="bc-measurebar" data-walkbar hidden>
      <output class="bc-measure-out" data-walk-out aria-live="polite"></output>
      <label><?= Lang::e('walk.height') ?> <input type="text" class="bc-input" inputmode="decimal" size="4" value="1.6" data-walk-height> m</label>
      <label><input type="checkbox" checked data-walk-gravity> <?= Lang::e('walk.gravity') ?></label>
      <label><input type="checkbox" checked data-walk-collide> <?= Lang::e('walk.collide') ?></label>
      <span class="bc-walkpad" role="group" aria-label="<?= Lang::e('walk.pad') ?>" data-walkpad>
        <button type="button" class="bc-tool" data-walk-key="w" title="<?= Lang::e('walk.fwd') ?>"><i class="bi bi-arrow-up" aria-hidden="true"></i><span class="visually-hidden"><?= Lang::e('walk.fwd') ?></span></button>
        <button type="button" class="bc-tool" data-walk-key="s" title="<?= Lang::e('walk.back') ?>"><i class="bi bi-arrow-down" aria-hidden="true"></i><span class="visually-hidden"><?= Lang::e('walk.back') ?></span></button>
        <button type="button" class="bc-tool" data-walk-key="a" title="<?= Lang::e('walk.left') ?>"><i class="bi bi-arrow-left" aria-hidden="true"></i><span class="visually-hidden"><?= Lang::e('walk.left') ?></span></button>
        <button type="button" class="bc-tool" data-walk-key="d" title="<?= Lang::e('walk.right') ?>"><i class="bi bi-arrow-right" aria-hidden="true"></i><span class="visually-hidden"><?= Lang::e('walk.right') ?></span></button>
        <button type="button" class="bc-tool" data-walk-key="e" title="<?= Lang::e('walk.up') ?>"><i class="bi bi-chevron-double-up" aria-hidden="true"></i><span class="visually-hidden"><?= Lang::e('walk.up') ?></span></button>
        <button type="button" class="bc-tool" data-walk-key="q" title="<?= Lang::e('walk.down') ?>"><i class="bi bi-chevron-double-down" aria-hidden="true"></i><span class="visually-hidden"><?= Lang::e('walk.down') ?></span></button>
      </span>
      <button type="button" class="btn-line btn-sm-cc" data-walk-done><?= Lang::e('measure.done') ?></button>
    </div>
    <div class="bc-measurebar" data-measurebar hidden>
      <button type="button" class="bc-tool" data-measure-mode="distance" aria-pressed="true"><i class="bi bi-arrows" aria-hidden="true"></i><span><?= Lang::e('measure.distance') ?></span></button>
      <button type="button" class="bc-tool" data-measure-mode="area" aria-pressed="false"><i class="bi bi-bounding-box-circles" aria-hidden="true"></i><span><?= Lang::e('measure.area') ?></span></button>
      <output class="bc-measure-out" data-measure-out aria-live="polite"></output>
      <button type="button" class="btn-line btn-sm-cc" data-measure-clear><?= Lang::e('measure.clear') ?></button>
      <button type="button" class="btn-line btn-sm-cc" data-measure-done><?= Lang::e('measure.done') ?></button>
    </div>

    <div class="bc-card" data-rvt hidden></div>

    <div class="bc-dwg" data-dwg hidden>
      <div class="bc-toolbar bc-sheets" role="toolbar" aria-label="<?= Lang::e('ui.sheets.label') ?>" data-sheet-tabs></div>
      <?php /* 3D: bimcraft.js shows this bar and the 3D canvas when the drawing is switched to 3D (dwg-view3d.js, loaded then). */ ?>
      <div class="bc-toolbar bc-sheets bc-dwg3d-bar" role="toolbar" aria-label="<?= Lang::e('ui.dwg3d.label') ?>" data-dwg3d-bar hidden>
        <button type="button" class="bc-tool" data-v3="iso" title="<?= Lang::e('ui.v3.iso.title') ?>"><i class="bi bi-box" aria-hidden="true"></i><span><?= Lang::e('ui.v3.iso') ?></span></button>
        <button type="button" class="bc-tool" data-v3="top" title="<?= Lang::e('ui.view.top.title') ?>"><i class="bi bi-square" aria-hidden="true"></i><span><?= Lang::e('viewcube.face.top') ?></span></button>
        <button type="button" class="bc-tool" data-v3="bottom" title="<?= Lang::e('ui.v3.bottom.title') ?>"><i class="bi bi-square-fill" aria-hidden="true"></i><span><?= Lang::e('viewcube.face.bottom') ?></span></button>
        <button type="button" class="bc-tool" data-v3="front" title="<?= Lang::e('ui.view.front.title') ?>"><i class="bi bi-layout-sidebar-inset" aria-hidden="true"></i><span><?= Lang::e('viewcube.face.front') ?></span></button>
        <button type="button" class="bc-tool" data-v3="back" title="<?= Lang::e('ui.v3.back.title') ?>"><i class="bi bi-layout-sidebar" aria-hidden="true"></i><span><?= Lang::e('viewcube.face.back') ?></span></button>
        <button type="button" class="bc-tool" data-v3="left" title="<?= Lang::e('ui.v3.left.title') ?>"><i class="bi bi-layout-split" aria-hidden="true"></i><span><?= Lang::e('viewcube.face.left') ?></span></button>
        <button type="button" class="bc-tool" data-v3="right" title="<?= Lang::e('ui.v3.right.title') ?>"><i class="bi bi-layout-sidebar-inset-reverse" aria-hidden="true"></i><span><?= Lang::e('viewcube.face.right') ?></span></button>
        <span class="bc-tool-sep" aria-hidden="true"></span>
        <button type="button" class="bc-tool" data-v3="persp" aria-pressed="true" title="<?= Lang::e('ui.v3.persp.title') ?>"><i class="bi bi-camera" aria-hidden="true"></i><span><?= Lang::e('ui.v3.persp') ?></span></button>
        <button type="button" class="bc-tool" data-v3="extents" title="<?= Lang::e('ui.v3.extents.title') ?>"><i class="bi bi-arrows-fullscreen" aria-hidden="true"></i><span><?= Lang::e('ui.v3.extents') ?></span></button>
        <span class="bc-tool-sep" aria-hidden="true"></span>
        <button type="button" class="bc-tool" data-v3="section" aria-pressed="false" title="<?= Lang::e('sec.dwg.title') ?>"><i class="bi bi-scissors" aria-hidden="true"></i><span><?= Lang::e('sec.button') ?></span></button>
      </div>
      <canvas data-dwg-canvas aria-label="<?= Lang::e('ui.dwg.canvas.label') ?>"></canvas>
      <div class="bc-dwg3d" data-dwg3d hidden><canvas data-dwg3d-canvas aria-label="<?= Lang::e('ui.dwg3d.canvas.label') ?>"></canvas></div>
      <p class="bc-coords" data-coords aria-hidden="true"></p>
      <div class="bc-equip" data-equip hidden></div>
    </div>

    <div class="bc-card bc-folder" data-folder hidden></div>

    <div class="bc-empty" data-empty>
      <h1 class="bc-empty-h"><?= Lang::e('ui.empty.title') ?></h1>
      <p class="bc-empty-lede"><?= Lang::e('ui.empty.lede') ?></p>
      <p class="bc-empty-cta">
        <button type="button" class="btn-plot" data-pick><i class="bi bi-file-earmark" aria-hidden="true"></i> <?= Lang::e('ui.choose.file') ?></button>
        <button type="button" class="btn-line" data-pick-folder><i class="bi bi-folder2-open" aria-hidden="true"></i> <?= Lang::e('ui.open.folder') ?></button>
      </p>
      <ul class="bc-formats">
        <li>
          <p class="bc-fmt-name">IFC</p><?php /* i18n-ignore: file format name */ ?>
          <p><?= Lang::e('ui.fmt.ifc') ?></p>
        </li>
        <li>
          <p class="bc-fmt-name"><?= Lang::e('ui.fmt.rvt.name') ?></p>
          <p><?= Lang::e('ui.fmt.rvt') ?></p>
        </li>
        <li>
          <p class="bc-fmt-name"><?= Lang::e('ui.fmt.dwg.name') ?></p>
          <p><?= Lang::e('ui.fmt.dwg') ?></p>
        </li>
        <li>
          <p class="bc-fmt-name"><?= Lang::e('ui.fmt.folder.name') ?></p>
          <p><?= Lang::e('ui.fmt.folder') ?></p>
        </li>
      </ul>
      <noscript><p class="bc-noscript"><?= Lang::e('ui.noscript') ?></p></noscript>
    </div>

    <p class="bc-status" role="status" aria-live="polite" data-status></p>
    <div class="bc-dropveil" data-dropveil hidden><p><?= Lang::e('ui.dropveil') ?></p></div>
  </section>

  <aside class="bc-panel bc-right" aria-label="<?= Lang::e('ui.right.label') ?>">
    <?php /* an assistant is connected (0.4 X.7, AGENTS-SPEC §7): bimcraft.js fills and shows it; never over the model */ ?>
    <div class="bc-assist-banner" role="status" data-assist-banner hidden><p></p><button type="button" class="btn-line btn-sm-cc" data-assist-disconnect><?= Lang::e('assist.disconnect') ?></button></div>
    <div class="bc-rtabs" role="tablist" aria-label="<?= Lang::e('ui.right.tabs') ?>">
      <button type="button" role="tab" id="bc-tab-props" class="bc-rtab" aria-selected="true" aria-controls="bc-pane-props" data-rtab="props"><?= Lang::e('ui.properties') ?></button>
      <button type="button" role="tab" id="bc-tab-color" class="bc-rtab" aria-selected="false" aria-controls="bc-pane-color" data-rtab="color" tabindex="-1"><?= Lang::e('color.tab') ?></button>
      <button type="button" role="tab" id="bc-tab-issues" class="bc-rtab" aria-selected="false" aria-controls="bc-pane-issues" data-rtab="issues" tabindex="-1"><?= Lang::e('bcf.tab') ?></button>
      <button type="button" role="tab" id="bc-tab-lib" class="bc-rtab" aria-selected="false" aria-controls="bc-pane-lib" data-rtab="library" tabindex="-1"><?= Lang::e('ui.library') ?></button>
    </div>
    <section class="bc-sec" id="bc-pane-props" role="tabpanel" aria-labelledby="bc-tab-props" data-rpane="props">
      <h2 class="bc-h"><?= Lang::e('ui.properties') ?></h2>
      <div data-props><p class="bc-muted"><?= Lang::e('ui.props.empty') ?></p></div>
    </section>
    <section class="bc-sec" id="bc-pane-color" role="tabpanel" aria-labelledby="bc-tab-color" data-rpane="color" hidden>
      <h2 class="bc-h"><?= Lang::e('color.title') ?></h2>
      <div data-color><p class="bc-muted"><?= Lang::e('color.needs.ifc') ?></p></div>
    </section>
    <section class="bc-sec" id="bc-pane-issues" role="tabpanel" aria-labelledby="bc-tab-issues" data-rpane="issues" hidden>
      <h2 class="bc-h"><?= Lang::e('bcf.title') ?></h2>
      <div data-issues><p class="bc-muted"><?= Lang::e('bcf.needs.ifc') ?></p></div>
      <input type="file" class="visually-hidden" accept=".bcf,.bcfzip" tabindex="-1" aria-hidden="true" data-bcf-input>
    </section>
    <section class="bc-sec" id="bc-pane-lib" role="tabpanel" aria-labelledby="bc-tab-lib" data-rpane="library" hidden>
      <h2 class="bc-h"><?= Lang::e('ui.library') ?></h2>
      <div data-library><p class="bc-muted"><?= Lang::e('ui.lib.nojs') ?></p></div>
    </section>
  </aside>
</main>
