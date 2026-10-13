<?php
// SPDX-License-Identifier: Apache-2.0
/**
 * settings.inc — BimCraft's own panel of the Settings dialog (core/components/app-settings.inc includes it, as the
 * "BimCraft" tab). bimcraft.js reads and stores the fields (localStorage bimcraft.settings), shows the stored values
 * each time the dialog opens, and resets them on "cc-settings-reset".
 *   bcSpaces  show rooms and spaces (IfcSpace) when a model opens            default off
 *   bcEdges   edges when a model opens: on / small models only / off          default on
 *   bcView    the view a model opens in: Fit / Top / Front / Side            default Fit
 *   bcSnap    wall snap is on when placing or pasting a library item starts   default off
 *   bcCorner  the corner indicator in the 3D views: view cube / axis icon / none   default view cube
 *   bcAssist, bcAssistRead, bcAssistEdit   AI assistants and scripts (0.4 X.7, AGENTS-SPEC §7): on/off, then which groups
 *             (read on, edit off when first turned on); the Activity list is filled by bimcraft.js        default off
 */
?>
<fieldset class="shell-set" data-bc-settings>
  <legend><?= Lang::e('set.open.legend') ?></legend>
  <label class="shell-check"><input type="checkbox" name="bcSpaces"> <?= Lang::e('ui.spaces.title') ?></label>
  <p class="shell-hint-text"><?= Lang::e('set.spaces.hint') ?></p>
  <div class="shell-seg" role="radiogroup" aria-label="<?= Lang::e('set.edges.label') ?>">
    <label><input type="radio" name="bcEdges" value="on"> <i class="bi bi-bounding-box" aria-hidden="true"></i> <?= Lang::e('set.edges.on') ?></label>
    <label><input type="radio" name="bcEdges" value="small"> <i class="bi bi-speedometer2" aria-hidden="true"></i> <?= Lang::e('set.edges.small') ?></label>
    <label><input type="radio" name="bcEdges" value="off"> <i class="bi bi-square" aria-hidden="true"></i> <?= Lang::e('set.edges.off') ?></label>
  </div>
  <p class="shell-hint-text"><?= Lang::e('set.edges.hint') ?></p>
</fieldset>
<fieldset class="shell-set">
  <legend><?= Lang::e('set.view') ?></legend>
  <div class="shell-seg bc-set-seg4" role="radiogroup" aria-label="<?= Lang::e('set.view') ?>">
    <label><input type="radio" name="bcView" value="iso"> <i class="bi bi-box" aria-hidden="true"></i> <?= Lang::e('ui.view.fit') ?></label>
    <label><input type="radio" name="bcView" value="top"> <i class="bi bi-square" aria-hidden="true"></i> <?= Lang::e('viewcube.face.top') ?></label>
    <label><input type="radio" name="bcView" value="front"> <i class="bi bi-layout-sidebar-inset" aria-hidden="true"></i> <?= Lang::e('viewcube.face.front') ?></label>
    <label><input type="radio" name="bcView" value="side"> <i class="bi bi-layout-sidebar-inset-reverse" aria-hidden="true"></i> <?= Lang::e('ui.view.side') ?></label>
  </div>
  <p class="shell-hint-text"><?= Lang::e('set.view.hint') ?></p>
</fieldset>
<fieldset class="shell-set">
  <legend><?= Lang::e('set.snap.legend') ?></legend>
  <label class="shell-check"><input type="checkbox" name="bcSnap"> <?= Lang::e('set.snap') ?></label>
  <p class="shell-hint-text"><?= Lang::e('set.snap.hint') ?></p>
</fieldset>
<fieldset class="shell-set">
  <legend><?= Lang::e('set.corner') ?></legend>
  <div class="shell-seg" role="radiogroup" aria-label="<?= Lang::e('set.corner') ?>">
    <label><input type="radio" name="bcCorner" value="cube"> <i class="bi bi-box" aria-hidden="true"></i> <?= Lang::e('viewcube.group') ?></label>
    <label><input type="radio" name="bcCorner" value="axes"> <i class="bi bi-arrows-move" aria-hidden="true"></i> <?= Lang::e('set.corner.axes') ?></label>
    <label><input type="radio" name="bcCorner" value="none"> <i class="bi bi-slash-circle" aria-hidden="true"></i> <?= Lang::e('checks.none') ?></label>
  </div>
  <p class="shell-hint-text"><?= Lang::e('set.corner.hint') ?></p>
</fieldset>
<fieldset class="shell-set" data-bc-assist-set>
  <legend><?= Lang::e('set.assist.legend') ?></legend>
  <label class="shell-check"><input type="checkbox" name="bcAssist"> <?= Lang::e('set.assist.on') ?></label>
  <p class="shell-hint-text"><?= Lang::e('set.assist.hint') ?></p>
  <label class="shell-check bc-set-sub"><input type="checkbox" name="bcAssistRead"> <?= Lang::e('set.assist.read') ?></label>
  <label class="shell-check bc-set-sub"><input type="checkbox" name="bcAssistEdit"> <?= Lang::e('set.assist.edit') ?></label>
  <details class="bc-assist-log" data-bc-assist-log>
    <summary><?= Lang::e('set.assist.activity') ?></summary>
    <ol></ol>
  </details>
</fieldset>
