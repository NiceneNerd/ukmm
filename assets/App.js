import { ModList } from "./components/ModList/ModList";
import { Log } from "./components/Log/Log";
import { MenuBar } from "./components/MenuBar/MenuBar";
import { Tabs, Tab } from "./components/Tabs/Tabs";
import { ProfileMenu } from "./components/ProfileMenu/ProfileMenu";
import { ModInfo } from "./components/ModInfo/ModInfo";
import { Toolbar } from "./components/Toolbar/Toolbar";
import { FolderView } from "./components/FolderView/FolderView";
import { DirtyBar } from "./components/DirtyBar/DirtyBar";

export class App extends Element {
  dirty = false;
  mods = [];
  currentMod = 0;
  profiles = [];
  currentProfile = "Default";
  log = [];

  constructor(props) {
    super(props);
    this.props = props;
    this.api = Window.this.xcall("GetApi");
    Window.this.api = this.api;
    this.handleToggle = this.handleToggle.bind(this);
    this.handleReorder = this.handleReorder.bind(this);
    this.handleLog = this.handleLog.bind(this);
    Window.this.log = this.handleLog;
    this.handleSelect = this.handleSelect.bind(this);
    this.handleOpen = this.handleOpen.bind(this);
  }

  componentDidMount() {
    const mods = this.api.mods();
    const profiles = this.api.profiles();
    const currentProfile = this.api.current_profile();
    this.componentUpdate({ mods, profiles, currentProfile });
  }

  handleToggle(mod) {
    mod.enabled = !mod.enabled;
    this.componentUpdate({ mods: this.mods.slice(), dirty: true });
  }

  handleReorder(oldIdxs, newIdx) {
    const modsToMove = oldIdxs.map((i) => this.mods[i]);
    for (const mod of modsToMove) {
      this.mods.splice(this.mods.indexOf(mod), 1);
    }
    const mods =
      newIdx == 0
        ? [...modsToMove, ...this.mods]
        : [...this.mods.slice(0, newIdx), ...modsToMove, ...this.mods.slice(newIdx)];
    this.componentUpdate({ mods, dirty: true });
  }

  handleSelect(index) {
    this.componentUpdate({ currentMod: index });
  }

  handleLog(record) {
    let log = this.log;
    log.push(record);
    this.componentUpdate({ log });
  }

  handleOpen(path) {
    console.log(path);
  }

  render() {
    return (
      <div style="flow: vertical; size: *;">
        <MenuBar />
        <frameset cols="*,36%" style="size: *;">
          <div style="size: *;">
            <Toolbar>
              <ProfileMenu
                currentProfile={this.currentProfile}
                profiles={this.profiles}
              />
              <div class="spacer"></div>
              <div class="counter">
                <strong>{this.mods.length}</strong> Mods /{" "}
                <strong>{this.mods.filter((m) => m.enabled).length} </strong>
                Active
              </div>
            </Toolbar>
            <frameset rows="*,15%" style="size: *;">
              <ModList
                mods={this.mods}
                onToggle={this.handleToggle}
                onReorder={this.handleReorder}
                onSelect={this.handleSelect}
              />
              {this.dirty ? <DirtyBar onApply={() => console.log("Hey")} /> : []}
              <splitter />
              <Log logs={this.log} />
            </frameset>
          </div>
          <splitter />
          <Tabs>
            <Tab label="Mod Info">
              <ModInfo mod={this.mods[this.currentMod]} />
            </Tab>
            <Tab label="Install">
              <FolderView onSelect={this.handleOpen} />
            </Tab>
          </Tabs>
        </frameset>
      </div>
    );
  }
}
