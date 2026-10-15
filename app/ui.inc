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
// "Takeoff" (0.5 part 4 RC.8, res-ui.js): the same rule for TakeoffCraft — else the button downloads the file instead.
$__takeoff = Catalog::app('takeoffcraft');
$__takeoffBuild = $__takeoff && Catalog::openIn($__takeoff, $__area['key']) ? AppBuilds::resolve($__takeoff['slug'], $__area['key']) : null;
$__takeoffUrl = $__takeoffBuild && in_array('handoff', $__takeoffBuild['manifest']['accepts'] ?? [], true) ? $__area['path'] . '/' . $__takeoff['slug'] : '';
// RVT preview (0.5 SG.8 b, rvt-convert.js): a spike, so the lab only — a release of this build keeps it off in /use. The
// library's pinned URL is handed to the page here, not through manifest.json (and written so the packers' vendor scan does
// not see it), so editions, the public repo and Studio never carry it before the owner's legal look (docs/spikes/rvt-preview.md).
$__rvtLib = 'rvt';
$__rvtVer = ($__area['key'] ?? '') === 'lab' ? basename((string)@readlink(dirname(__DIR__, 3) . "/vendor/$__rvtLib/current")) : '';
$__rvtPreview = $__rvtVer !== '' ? "/assets/vendor/$__rvtLib/$__rvtVer/rvt.js" : '';
?>
<main id="main" class="bc" data-bimcraft<?php if ($__editUrl): ?> data-edit-url="<?= e($__editUrl) ?>" data-edit-app="<?= e($__edit['name']) ?>"<?php endif; ?><?php if ($__rvtPreview): ?> data-rvt-preview="<?= e($__rvtPreview) ?>"<?php endif; ?>>
  <aside class="bc-panel bc-left" aria-label="<?= Lang::e('qto.sp.file') ?>">
    <section class="bc-sec">
      <h2 class="bc-h"><?= Lang::e('ui.open.title') ?></h2>
      <label class="bc-drop" for="bc-file" data-drop>
        <i class="bi bi-folder2-open" aria-hidden="true"></i>
        <span class="bc-drop-main"><?= Lang::e('ui.drop.main') ?></span>
        <span class="bc-drop-sub"><?= Lang::e('ui.drop.sub') ?></span>
        <input type="file" id="bc-file" class="visually-hidden" accept=".ifc,.rvt,.rfa,.rte,.rft,.dwg,.dxf,.pln,.pla,.tpl,.bpn,.las,.laz,.xyz,.pts" data-file>
      </label>
      <button type="button" class="btn-line bc-folder-btn" data-pick-folder><i class="bi bi-folder-symlink" aria-hidden="true"></i> <?= Lang::e('ui.open.folder') ?></button>
      <?php /* New building (0.5 part 3): bimcraft.js "New building", new-model.js */ ?>
      <button type="button" class="btn-line bc-folder-btn" data-new><i class="bi bi-building-add" aria-hidden="true"></i> <?= Lang::e('new.button') ?></button>
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
    <?php /* Levels (0.5 part 3, BM.2; model-ui.js renderLevels): shown while Draw is on — pick the plan's level, rename, elevation, height, add. */ ?>
    <section class="bc-sec" data-levels-sec hidden>
      <h2 class="bc-h"><?= Lang::e('model.levels') ?></h2>
      <label class="bc-lv-units"><span><?= Lang::e('model.units') ?></span> <select class="bc-mb-select" data-model-units>
        <option value="ftin"><?= Lang::e('model.units.ftin') ?></option>
        <option value="mm"><?= Lang::e('model.units.mm') ?></option>
      </select></label>
      <div data-levels></div>
      <p class="bc-note"><?= Lang::e('model.levels.note') ?></p>
    </section>
    <?php /* Schedules of what was drawn (0.5 BM.7; model-schedule.js): live, one table at a time, CSV. */ ?>
    <section class="bc-sec" data-msched-sec hidden>
      <h2 class="bc-h"><?= Lang::e('model.sch.title') ?></h2>
      <label class="bc-lv-units"><span class="visually-hidden"><?= Lang::e('model.sch.which') ?></span><select class="bc-mb-select" data-msched-kind aria-label="<?= Lang::e('model.sch.which') ?>">
        <option value="doors"><?= Lang::e('model.sch.doors') ?></option>
        <option value="windows"><?= Lang::e('model.sch.windows') ?></option>
        <option value="walls"><?= Lang::e('model.sch.walls') ?></option>
        <option value="rooms"><?= Lang::e('model.sch.rooms') ?></option>
      </select></label>
      <div class="bc-msched" data-msched></div>
      <button type="button" class="btn-line btn-sm-cc" data-msched-csv><i class="bi bi-download" aria-hidden="true"></i> <?= Lang::e('model.sch.csv') ?></button>
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
      <?php /* Model (0.5 part 3, track BM; model-ui.js): Draw opens the plan view of a level beside the 3D view, with the building tools. */ ?>
      <span class="bc-tool-sep" aria-hidden="true"></span>
      <button type="button" class="bc-tool" data-model-toggle aria-pressed="false" title="<?= Lang::e('model.draw.title') ?>"><i class="bi bi-bricks" aria-hidden="true"></i><span><?= Lang::e('model.draw') ?></span></button>
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
          <button type="button" role="menuitem" data-vis="secbox" title="<?= Lang::e('vis.secbox.title') ?>"><?= Lang::e('vis.secbox') ?> <kbd>Shift+X</kbd></button><?php /* i18n-ignore: key names */ ?>
          <hr>
          <button type="button" role="menuitem" data-vis="all" title="<?= Lang::e('vis.showall.title') ?>"><?= Lang::e('vis.showall') ?> <kbd>Shift+H</kbd></button><?php /* i18n-ignore: key names */ ?>
          <hr>
          <?php /* Point clouds (0.5 SG.1): bimcraft.js "Point clouds" + cloud-ui.js (its bar is built there). In this menu, so the toolbar keeps its two rows. */ ?>
          <button type="button" role="menuitem" data-cloud-open aria-pressed="false" title="<?= Lang::e('pc.tool.title') ?>"><?= Lang::e('pc.tool') ?></button>
          <?php /* Map layers on the ground (0.5 SG.3): bimcraft.js "Map layers" + maps-bim.js / maplayers.js. */ ?>
          <button type="button" role="menuitem" data-map-open title="<?= Lang::e('ml.bim.tool.title') ?>"><?= Lang::e('ml.bim.tool') ?></button>
        </div>
      </details>
      <span class="bc-tool-sep" aria-hidden="true"></span>
      <button type="button" class="bc-tool" data-toggle="section" aria-pressed="false" title="<?= Lang::e('ui.section.title') ?>"><i class="bi bi-scissors" aria-hidden="true"></i><span><?= Lang::e('sec.button') ?></span></button>
      <button type="button" class="bc-tool" data-toggle="walk" aria-pressed="false" title="<?= Lang::e('walk.title') ?>"><i class="bi bi-person-walking" aria-hidden="true"></i><span><?= Lang::e('walk.button') ?></span></button>
      <?php /* Render a picture (0.5 RC.11): bimcraft.js "Render a picture" → render-bim.js + the shared render/ (ModelCraft's, synced). */ ?>
      <button type="button" class="bc-tool" data-render-open title="<?= Lang::e('ui.render.title') ?>"><i class="bi bi-camera" aria-hidden="true"></i><span><?= Lang::e('ui.render') ?></span></button>
      <?php /* Paint materials (0.5 RC.7): bimcraft.js "Paint materials" → paint-ui-bim.js + paint-bim.js. */ ?>
      <button type="button" class="bc-tool" data-paint-open="paint" aria-pressed="false" title="<?= Lang::e('ui.paint.title') ?>"><i class="bi bi-paint-bucket" aria-hidden="true"></i><span><?= Lang::e('ui.paint') ?></span></button>
      <?php /* Views (0.5 RC.7): bimcraft.js "Views" → views-ui-bim.js + views-bim.js (saved cameras and walkthroughs in the IFC). */ ?>
      <button type="button" class="bc-tool" data-saved-views="views" aria-pressed="false" title="<?= Lang::e('ui.views.title') ?>"><i class="bi bi-camera-reels" aria-hidden="true"></i><span><?= Lang::e('ui.views') ?></span></button>
      <?php if ($__detailUrl): ?>
      <span class="bc-tool-sep" aria-hidden="true"></span>
      <button type="button" class="bc-tool" data-handoff-ifc data-handoff-url="<?= e($__detailUrl) ?>" data-handoff-app="<?= e($__detail['name']) ?>" title="<?= Lang::e('handoff.ifc.title', ['app' => $__detail['name']]) ?>"><i class="bi bi-box-arrow-up-right" aria-hidden="true"></i><span><?= Lang::e('handoff.ifc.button', ['app' => $__detail['name']]) ?></span></button>
      <?php endif; ?>
      <button type="button" class="bc-tool" data-toggle="measure" aria-pressed="false" title="<?= Lang::e('measure.title') ?>"><i class="bi bi-rulers" aria-hidden="true"></i><span><?= Lang::e('measure.button') ?></span></button>
      <input type="file" class="visually-hidden" accept=".las,.laz,.xyz,.pts" tabindex="-1" aria-hidden="true" data-cloud-input>
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
        <?php /* Free-form volumes (0.5 WL.1; section-volume.js): several boxes (kept: inside any), tilted faces, extra planes. */ ?>
        <span class="bc-sec-cells">
          <label><span class="visually-hidden"><?= Lang::e('sec.cell.label') ?></span><select class="select-cc bc-select" data-sec-cell></select></label>
          <button type="button" class="btn-line btn-sm-cc" data-sec-cell-add title="<?= Lang::e('sec.cell.add.title') ?>"><?= Lang::e('sec.cell.add') ?></button>
          <button type="button" class="btn-line btn-sm-cc" data-sec-cell-remove hidden><?= Lang::e('sec.cell.remove') ?></button>
        </span>
        <details class="bc-sec-shape" data-sec-shape>
          <summary><?= Lang::e('sec.shape') ?></summary>
          <span class="bc-sec-shape-body">
            <span class="bc-sec-tilt">
              <label><?= Lang::e('sec.tilt.face') ?> <select class="select-cc bc-select" data-sec-tilt-face>
                <?php foreach (['x0', 'x1', 'y0', 'y1', 'z0', 'z1'] as $__f): ?><option value="<?= $__f ?>"><?= Lang::e('sec.face.' . $__f) ?></option><?php endforeach; ?>
              </select></label>
              <label><span data-sec-tilt-alabel></span> <input type="number" class="bc-input bc-num" min="-80" max="80" step="1" value="0" data-sec-tilt-a> °</label>
              <label><span data-sec-tilt-blabel></span> <input type="number" class="bc-input bc-num" min="-80" max="80" step="1" value="0" data-sec-tilt-b> °</label>
              <button type="button" class="btn-line btn-sm-cc" data-sec-tilt-level title="<?= Lang::e('sec.tilt.level.title') ?>"><?= Lang::e('sec.tilt.level') ?></button>
            </span>
            <span class="bc-sec-planes" data-sec-planes></span>
            <button type="button" class="btn-line btn-sm-cc" data-sec-plane-add title="<?= Lang::e('sec.plane.add.title') ?>"><?= Lang::e('sec.plane.add') ?></button>
            <span class="bc-sub" data-sec-shape-note></span>
          </span>
        </details>
      </span>
      <?php /* Sections square to a building turned in its site (0.5 BM.9): shown when the model's walls run at an angle to its axes. */ ?>
      <button type="button" class="bc-tool" data-sec-turn aria-pressed="false" hidden title="<?= Lang::e('sec.turn.title') ?>"><i class="bi bi-bounding-box-circles" aria-hidden="true"></i><span><?= Lang::e('sec.turn') ?></span></button>
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
    <?php /* Plan view + model bar (0.5 part 3, track BM): model-ui.js / model-planview.js. Beside the 3D view (Split) or alone (Plan). */ ?>
    <div class="bc-plan" data-plan-wrap hidden>
      <canvas data-plan-canvas aria-label="<?= Lang::e('model.plan.label') ?>"></canvas>
      <div class="bc-modelbar" role="toolbar" aria-label="<?= Lang::e('model.bar.label') ?>" data-modelbar>
        <div class="bc-modelbar-row">
          <label><span class="visually-hidden"><?= Lang::e('model.level') ?></span><select class="bc-mb-select" data-model-level aria-label="<?= Lang::e('model.level') ?>"></select></label>
          <span class="bc-mb-tools">
            <button type="button" class="bc-tool" data-mtool="select" aria-pressed="true" title="<?= Lang::e('model.tool.select.title') ?>"><i class="bi bi-cursor" aria-hidden="true"></i><span><?= Lang::e('model.tool.select') ?></span></button>
            <button type="button" class="bc-tool" data-mtool="wall" aria-pressed="false" title="<?= Lang::e('model.tool.wall.title') ?>"><i class="bi bi-bricks" aria-hidden="true"></i><span><?= Lang::e('model.tool.wall') ?></span></button>
            <button type="button" class="bc-tool" data-mtool="door" aria-pressed="false" title="<?= Lang::e('model.tool.door.title') ?>"><i class="bi bi-door-open" aria-hidden="true"></i><span><?= Lang::e('model.tool.door') ?></span></button>
            <button type="button" class="bc-tool" data-mtool="window" aria-pressed="false" title="<?= Lang::e('model.tool.window.title') ?>"><i class="bi bi-window" aria-hidden="true"></i><span><?= Lang::e('model.tool.window') ?></span></button>
            <button type="button" class="bc-tool" data-mtool="floor" aria-pressed="false" title="<?= Lang::e('model.tool.floor.title') ?>"><i class="bi bi-layers" aria-hidden="true"></i><span><?= Lang::e('model.tool.floor') ?></span></button>
            <button type="button" class="bc-tool" data-mtool="ceiling" aria-pressed="false" title="<?= Lang::e('model.tool.ceiling.title') ?>"><i class="bi bi-layers-half" aria-hidden="true"></i><span><?= Lang::e('model.tool.ceiling') ?></span></button>
            <button type="button" class="bc-tool" data-mtool="roof" aria-pressed="false" title="<?= Lang::e('model.tool.roof.title') ?>"><i class="bi bi-house" aria-hidden="true"></i><span><?= Lang::e('model.tool.roof') ?></span></button>
            <?php /* The residential tools (0.5 part 4, track RC; res-ui.js through model-ui.js extend()). */ ?>
            <details class="bc-mb-more" data-res-tools>
              <summary class="bc-tool" title="<?= Lang::e('res.tools.title') ?>"><i class="bi bi-tools" aria-hidden="true"></i><span><?= Lang::e('res.tools') ?></span></summary>
              <div class="bc-mb-menu" role="group" aria-label="<?= Lang::e('res.tools') ?>">
                <button type="button" class="bc-tool" data-mtool="roofplane" aria-pressed="false" title="<?= Lang::e('res.tool.roofplane.title') ?>"><i class="bi bi-triangle" aria-hidden="true"></i><span><?= Lang::e('res.tool.roofplane') ?></span></button>
                <button type="button" class="bc-tool" data-mtool="foundation" aria-pressed="false" title="<?= Lang::e('res.tool.foundation.title') ?>"><i class="bi bi-bounding-box" aria-hidden="true"></i><span><?= Lang::e('res.tool.foundation') ?></span></button>
                <button type="button" class="bc-tool" data-mtool="countertop" aria-pressed="false" title="<?= Lang::e('res.tool.countertop.title') ?>"><i class="bi bi-square-half" aria-hidden="true"></i><span><?= Lang::e('res.tool.countertop') ?></span></button>
                <button type="button" class="bc-tool" data-mtool="stair" aria-pressed="false" title="<?= Lang::e('res.tool.stair.title') ?>"><i class="bi bi-reception-4" aria-hidden="true"></i><span><?= Lang::e('res.tool.stair') ?></span></button>
                <button type="button" class="bc-tool" data-mtool="railing" aria-pressed="false" title="<?= Lang::e('res.tool.railing.title') ?>"><i class="bi bi-distribute-horizontal" aria-hidden="true"></i><span><?= Lang::e('res.tool.railing') ?></span></button>
                <button type="button" class="bc-tool" data-mtool="framing" aria-pressed="false" title="<?= Lang::e('res.tool.framing.title') ?>"><i class="bi bi-grid-3x3" aria-hidden="true"></i><span><?= Lang::e('res.tool.framing') ?></span></button>
                <button type="button" class="bc-tool" data-mtool="terrain" aria-pressed="false" title="<?= Lang::e('res.tool.terrain.title') ?>"><i class="bi bi-image-alt" aria-hidden="true"></i><span><?= Lang::e('res.tool.terrain') ?></span></button>
                <button type="button" class="bc-tool" data-mtool="pad" aria-pressed="false" title="<?= Lang::e('res.tool.pad.title') ?>"><i class="bi bi-square" aria-hidden="true"></i><span><?= Lang::e('res.tool.pad') ?></span></button>
                <button type="button" class="bc-tool" data-mtool="paving" aria-pressed="false" title="<?= Lang::e('res.tool.paving.title') ?>"><i class="bi bi-signpost-split" aria-hidden="true"></i><span><?= Lang::e('res.tool.paving') ?></span></button>
                <button type="button" class="bc-tool" data-mtool="section" aria-pressed="false" title="<?= Lang::e('res.tool.section.title') ?>"><i class="bi bi-slash-square" aria-hidden="true"></i><span><?= Lang::e('res.tool.section') ?></span></button>
                <button type="button" class="bc-tool" data-mtool="electrical" aria-pressed="false" title="<?= Lang::e('res.tool.electrical.title') ?>"><i class="bi bi-plug" aria-hidden="true"></i><span><?= Lang::e('res.tool.electrical') ?></span></button>
              </div>
            </details>
          </span>
          <span class="bc-mb-right">
            <button type="button" class="bc-tool" data-model-dims aria-pressed="true" title="<?= Lang::e('model.dims.title') ?>"><i class="bi bi-arrows" aria-hidden="true"></i><span class="visually-hidden"><?= Lang::e('model.dims') ?></span></button>
            <button type="button" class="bc-tool" data-model-print title="<?= Lang::e('model.print.title') ?>"><i class="bi bi-printer" aria-hidden="true"></i><span class="visually-hidden"><?= Lang::e('model.print') ?></span></button>
            <button type="button" class="bc-tool" data-res-sheetsopen title="<?= Lang::e('res.sh.open.title') ?>"><i class="bi bi-files" aria-hidden="true"></i><span class="visually-hidden"><?= Lang::e('res.sh.title') ?></span></button>
            <button type="button" class="bc-tool" data-res-takeoff data-handoff-url="<?= e($__takeoffUrl) ?>" title="<?= Lang::e('res.to.title') ?>"><i class="bi bi-calculator" aria-hidden="true"></i><span class="visually-hidden"><?= Lang::e('res.to.button') ?></span></button>
            <button type="button" class="bc-tool" data-model-fit title="<?= Lang::e('model.fit.title') ?>"><i class="bi bi-arrows-fullscreen" aria-hidden="true"></i><span class="visually-hidden"><?= Lang::e('model.fit') ?></span></button>
            <span class="bc-mb-layout" role="group" aria-label="<?= Lang::e('model.layout') ?>">
              <button type="button" class="bc-tool" data-model-layout="plan" aria-pressed="false"><?= Lang::e('model.layout.plan') ?></button>
              <button type="button" class="bc-tool" data-model-layout="split" aria-pressed="true"><?= Lang::e('model.layout.split') ?></button>
              <button type="button" class="bc-tool" data-model-layout="3d" aria-pressed="false" title="<?= Lang::e('model.layout.3d.title') ?>"><?= Lang::e('model.layout.3d') ?></button>
            </span>
          </span>
        </div>
        <div class="bc-modelbar-row" data-mopts="wall" hidden>
          <label><span class="visually-hidden"><?= Lang::e('model.wall.type') ?></span><select class="bc-mb-select" data-model-walltype aria-label="<?= Lang::e('model.wall.type') ?>"></select></label>
          <label><span class="visually-hidden"><?= Lang::e('model.wall.top') ?></span><select class="bc-mb-select" data-model-walltop aria-label="<?= Lang::e('model.wall.top') ?>">
            <option value="level"><?= Lang::e('model.wall.top.level') ?></option>
            <option value="fixed"><?= Lang::e('model.wall.top.fixed') ?></option>
          </select></label>
          <input type="text" class="bc-input bc-num" data-model-wallh aria-label="<?= Lang::e('model.wall.height') ?>" size="7">
        </div>
        <?php foreach (['door', 'window'] as $__k): ?>
        <div class="bc-modelbar-row" data-mopts="<?= $__k ?>" hidden>
          <label><span class="visually-hidden"><?= Lang::e('model.ot.type') ?></span><select class="bc-mb-select" data-model-otype="<?= $__k ?>" aria-label="<?= Lang::e('model.ot.type') ?>"></select></label>
          <label><span><?= Lang::e('model.ot.w') ?></span> <input type="text" class="bc-input bc-num" data-model-ow="<?= $__k ?>" aria-label="<?= Lang::e('model.ot.width') ?>" size="7"></label>
          <label><span><?= Lang::e('model.ot.h') ?></span> <input type="text" class="bc-input bc-num" data-model-oh="<?= $__k ?>" aria-label="<?= Lang::e('model.ot.height') ?>" size="7"></label>
          <?php if ($__k === 'window'): ?><label><span><?= Lang::e('model.ot.sill') ?></span> <input type="text" class="bc-input bc-num" data-model-osill aria-label="<?= Lang::e('model.ot.sill.label') ?>" size="7"></label><?php endif; ?>
          <button type="button" class="bc-tool" data-model-flip title="<?= Lang::e('model.ot.flip.title') ?>"><i class="bi bi-symmetry-vertical" aria-hidden="true"></i><span><?= Lang::e('model.ot.flip') ?></span></button>
        </div>
        <?php endforeach; ?>
        <div class="bc-modelbar-row" data-mopts="floor" hidden>
          <label><span class="visually-hidden"><?= Lang::e('model.fl.mode') ?></span><select class="bc-mb-select" data-model-flmode="floor" aria-label="<?= Lang::e('model.fl.mode') ?>">
            <option value="room"><?= Lang::e('model.fl.room') ?></option>
            <option value="building"><?= Lang::e('model.fl.building') ?></option>
          </select></label>
          <label><span class="visually-hidden"><?= Lang::e('model.fl.kind') ?></span><select class="bc-mb-select" data-model-flkind aria-label="<?= Lang::e('model.fl.kind') ?>">
            <option value="wood"><?= Lang::e('model.floor.wood') ?></option>
            <option value="concrete"><?= Lang::e('model.floor.concrete') ?></option>
          </select></label>
          <label><span><?= Lang::e('model.prop.thickness') ?></span> <input type="text" class="bc-input bc-num" data-model-flthick size="7"></label>
        </div>
        <div class="bc-modelbar-row" data-mopts="ceiling" hidden>
          <label><span class="visually-hidden"><?= Lang::e('model.fl.mode') ?></span><select class="bc-mb-select" data-model-flmode="ceiling" aria-label="<?= Lang::e('model.fl.mode') ?>">
            <option value="room"><?= Lang::e('model.fl.room') ?></option>
            <option value="building"><?= Lang::e('model.fl.building') ?></option>
          </select></label>
          <label><span><?= Lang::e('model.ce.height') ?></span> <input type="text" class="bc-input bc-num" data-model-ceheight size="7"></label>
        </div>
        <div class="bc-modelbar-row" data-mopts="roof" hidden>
          <label><span><?= Lang::e('model.rf.pitch') ?></span> <input type="text" class="bc-input bc-num" data-model-rfpitch size="6"></label>
          <label><span><?= Lang::e('model.rf.overhang') ?></span> <input type="text" class="bc-input bc-num" data-model-rfover size="7"></label>
          <label><span class="visually-hidden"><?= Lang::e('model.rf.ends') ?></span><select class="bc-mb-select" data-model-rfends aria-label="<?= Lang::e('model.rf.ends') ?>">
            <option value="hip"><?= Lang::e('model.rf.hip') ?></option>
            <option value="gable-short"><?= Lang::e('model.rf.gable.short') ?></option>
            <option value="gable-long"><?= Lang::e('model.rf.gable.long') ?></option>
            <option value="shed"><?= Lang::e('res.rf.shed.near') ?></option>
          </select></label>
          <label><input type="checkbox" data-model-rffascia checked> <?= Lang::e('res.rf.fasciasoffit') ?></label>
        </div>
        <div class="bc-modelbar-row" data-mopts="foundation" hidden>
          <label><span class="visually-hidden"><?= Lang::e('res.fd.kind') ?></span><select class="bc-mb-select" data-res-fdkind aria-label="<?= Lang::e('res.fd.kind') ?>">
            <?php foreach (['slab', 'stemslab', 'crawl', 'basement'] as $__fk): ?><option value="<?= $__fk ?>"><?= Lang::e('res.fd.kind.' . $__fk) ?></option><?php endforeach; ?>
          </select></label>
        </div>
        <div class="bc-modelbar-row" data-mopts="countertop" hidden>
          <label><span><?= Lang::e('res.ct.overhang') ?></span> <input type="text" class="bc-input bc-num" data-res-ctover size="6"></label>
          <label><span><?= Lang::e('model.prop.thickness') ?></span> <input type="text" class="bc-input bc-num" data-res-ctthick size="6"></label>
        </div>
        <div class="bc-modelbar-row" data-mopts="stair" hidden>
          <label><span class="visually-hidden"><?= Lang::e('res.st.kind') ?></span><select class="bc-mb-select" data-res-stkind aria-label="<?= Lang::e('res.st.kind') ?>">
            <?php foreach (['straight', 'l', 'u', 'winder'] as $__sk): ?><option value="<?= $__sk ?>"><?= Lang::e('res.st.kind.' . $__sk) ?></option><?php endforeach; ?>
          </select></label>
          <label><span class="visually-hidden"><?= Lang::e('res.st.turn') ?></span><select class="bc-mb-select" data-res-stturn aria-label="<?= Lang::e('res.st.turn') ?>">
            <option value="left"><?= Lang::e('res.st.turn.left') ?></option><option value="right"><?= Lang::e('res.st.turn.right') ?></option>
          </select></label>
          <label><span><?= Lang::e('res.st.width') ?></span> <input type="text" class="bc-input bc-num" data-res-stwidth size="6"></label>
          <label><span class="visually-hidden"><?= Lang::e('res.st.railings') ?></span><select class="bc-mb-select" data-res-strail aria-label="<?= Lang::e('res.st.railings') ?>">
            <?php foreach (['both', 'left', 'right', 'none'] as $__sr): ?><option value="<?= $__sr ?>"><?= Lang::e('res.st.rail.' . $__sr) ?></option><?php endforeach; ?>
          </select></label>
        </div>
        <div class="bc-modelbar-row" data-mopts="railing" hidden>
          <label><span class="visually-hidden"><?= Lang::e('res.rl.kind') ?></span><select class="bc-mb-select" data-res-rlkind aria-label="<?= Lang::e('res.rl.kind') ?>">
            <option value="guard"><?= Lang::e('res.rail.guard') ?></option><option value="handrail"><?= Lang::e('res.rail.handrail') ?></option>
          </select></label>
          <label><span><?= Lang::e('res.rl.height') ?></span> <input type="text" class="bc-input bc-num" data-res-rlheight size="6"></label>
        </div>
        <div class="bc-modelbar-row" data-mopts="framing" hidden>
          <label><span class="visually-hidden"><?= Lang::e('res.fr.spacing') ?></span><select class="bc-mb-select" data-res-frsp aria-label="<?= Lang::e('res.fr.spacing') ?>"></select></label>
          <label><span class="visually-hidden"><?= Lang::e('res.fr.corners') ?></span><select class="bc-mb-select" data-res-frcorner aria-label="<?= Lang::e('res.fr.corners') ?>">
            <option value="three-stud"><?= Lang::e('res.fr.corner.three') ?></option><option value="california"><?= Lang::e('res.fr.corner.california') ?></option>
          </select></label>
          <label><span class="visually-hidden"><?= Lang::e('res.fr.roofmode') ?></span><select class="bc-mb-select" data-res-frroof aria-label="<?= Lang::e('res.fr.roofmode') ?>">
            <option value="stick"><?= Lang::e('res.fr.roof.stick') ?></option><option value="truss"><?= Lang::e('res.fr.roof.truss') ?></option>
          </select></label>
          <label><input type="checkbox" data-res-frblock checked> <?= Lang::e('res.fr.blocking.on') ?></label>
          <button type="button" class="bc-tool" data-res-frall><?= Lang::e('res.fr.all') ?></button>
          <button type="button" class="bc-tool" data-res-frshow aria-pressed="true"><?= Lang::e('res.fr.show') ?></button>
          <button type="button" class="bc-tool" data-res-frsee aria-pressed="false"><?= Lang::e('res.fr.see') ?></button>
        </div>
        <div class="bc-modelbar-row" data-mopts="terrain" hidden>
          <label class="btn-line btn-sm-cc"><input type="file" data-res-trfile hidden accept=".csv,.txt,.xyz,.pnezd,.dxf,.dwg,.las,.laz,.tif,.tiff,.asc"><i class="bi bi-folder2-open" aria-hidden="true"></i> <?= Lang::e('res.tr.open') ?></label>
          <label><span class="visually-hidden"><?= Lang::e('res.tr.unit') ?></span><select class="bc-mb-select" data-res-trunit aria-label="<?= Lang::e('res.tr.unit') ?>">
            <option value="ft"><?= Lang::e('res.tr.unit.ft') ?></option><option value="ftUS"><?= Lang::e('res.tr.unit.ftus') ?></option><option value="m"><?= Lang::e('res.tr.unit.m') ?></option><option value="mm"><?= Lang::e('res.tr.unit.mm') ?></option>
          </select></label>
          <span class="bc-sub"><?= Lang::e('res.tr.or') ?></span>
          <label><span><?= Lang::e('res.tr.spot') ?></span> <input type="text" class="bc-input bc-num" data-res-trz size="7"></label>
          <button type="button" class="bc-tool" data-res-trmake><?= Lang::e('res.tr.make') ?></button>
          <button type="button" class="bc-tool" data-res-trclear><?= Lang::e('res.tr.clear') ?></button>
          <button type="button" class="bc-tool" data-res-trxml title="<?= Lang::e('res.tr.xml.title') ?>"><?= Lang::e('res.tr.xml') ?></button>
        </div>
        <div class="bc-modelbar-row" data-mopts="pad" hidden>
          <label><span><?= Lang::e('res.pad.margin') ?></span> <input type="text" class="bc-input bc-num" data-res-padmargin size="7"></label>
          <label><span><?= Lang::e('res.pad.depth') ?></span> <input type="text" class="bc-input bc-num" data-res-paddepth size="7"></label>
          <label><span><?= Lang::e('res.pad.slope') ?></span> <input type="text" class="bc-input bc-num" data-res-padslope size="5"></label>
        </div>
        <div class="bc-modelbar-row" data-mopts="electrical" hidden>
          <label><span class="visually-hidden"><?= Lang::e('res.el.device') ?></span><select class="bc-mb-select" data-res-elkind aria-label="<?= Lang::e('res.el.device') ?>"></select></label>
          <label><span><?= Lang::e('res.el.sch.circuit') ?></span> <input type="text" class="bc-input bc-num" data-res-elcircuit size="4" maxlength="20"></label>
          <label><span><?= Lang::e('res.el.height') ?></span> <input type="text" class="bc-input bc-num" data-res-elheight size="7"></label>
        </div>
        <div class="bc-modelbar-row" data-mopts="ellink" hidden>
          <span class="bc-sub"><?= Lang::e('res.el.link.row') ?></span>
          <button type="button" class="btn-line btn-sm-cc" data-res-eldone><?= Lang::e('res.el.done') ?></button>
        </div>
        <div class="bc-modelbar-row" data-mopts="paving" hidden>
          <label><span class="visually-hidden"><?= Lang::e('res.hard.kind') ?></span><select class="bc-mb-select" data-res-hdkind aria-label="<?= Lang::e('res.hard.kind') ?>">
            <option value="walk"><?= Lang::e('res.hard.walk') ?></option><option value="driveway"><?= Lang::e('res.hard.driveway') ?></option><option value="patio"><?= Lang::e('res.hard.patio') ?></option>
          </select></label>
        </div>
        <div class="bc-modelbar-row" data-mopts="roofplane" hidden>
          <label><span><?= Lang::e('model.rf.pitch') ?></span> <input type="text" class="bc-input bc-num" data-res-rppitch size="6"></label>
          <label><span><?= Lang::e('res.rp.base') ?></span> <input type="text" class="bc-input bc-num" data-res-rpbase size="8"></label>
        </div>
        <div class="bc-modelbar-row" data-mtyped-row hidden>
          <label class="bc-mb-typed"><span><?= Lang::e('model.typed') ?></span> <input type="text" class="bc-input" data-model-typed size="12" autocomplete="off" spellcheck="false" aria-describedby="bc-model-hint"></label>
          <span class="bc-sub" id="bc-model-hint" data-model-hint aria-live="polite"></span>
        </div>
      </div>
    </div>
    <?php /* Sheets (0.5 part 4 RC.5; res-ui.js openSheets): the drawing set made from the model. */ ?>
    <dialog class="bc-sheets" data-res-sheets-dlg aria-labelledby="bc-sheets-h">
      <div class="bc-sheets-head">
        <h2 id="bc-sheets-h" class="bc-h"><?= Lang::e('res.sh.title') ?></h2>
        <label><span><?= Lang::e('res.sh.paper') ?></span> <select class="bc-mb-select" data-res-shpaper>
          <option value="letter"><?= Lang::e('res.sh.paper.letter') ?></option><option value="tabloid"><?= Lang::e('res.sh.paper.tabloid') ?></option><option value="archd"><?= Lang::e('res.sh.paper.archd') ?></option><option value="a3">A3</option><option value="a1">A1</option>
        </select></label>
        <button type="button" class="btn-plot btn-sm-cc" data-res-shpdf><i class="bi bi-download" aria-hidden="true"></i> <?= Lang::e('res.sh.pdf') ?></button>
        <button type="button" class="btn-line btn-sm-cc" data-res-shclose><?= Lang::e('res.sh.close') ?></button>
      </div>
      <div class="bc-sheets-body">
        <ol class="bc-sheets-list" data-res-shlist></ol>
        <div class="bc-sheets-view" data-res-shview></div>
      </div>
    </dialog>
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
      <button type="button" class="bc-tool" data-measure-mode="point" aria-pressed="false" title="<?= Lang::e('measure.point.title') ?>"><i class="bi bi-geo-alt" aria-hidden="true"></i><span><?= Lang::e('measure.point') ?></span></button>
      <output class="bc-measure-out" data-measure-out aria-live="polite"></output>
      <button type="button" class="btn-line btn-sm-cc" data-measure-copy hidden><?= Lang::e('measure.point.copy') ?></button>
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
        <button type="button" class="btn-line" data-new><i class="bi bi-building-add" aria-hidden="true"></i> <?= Lang::e('new.button') ?></button>
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
      <input type="file" class="visually-hidden" accept=".json,application/json" tabindex="-1" aria-hidden="true" data-color-input>
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
