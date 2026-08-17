<script setup lang="ts">
import { inject, ref, computed, onMounted, onBeforeUnmount, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import PageShell from "@/components/PageShell.vue";
import WinTextBlock from "@winui/components/WinTextBlock.vue";
import WinButton from "@winui/components/WinButton.vue";
import WinTextBox from "@winui/components/WinTextBox.vue";
import WinHyperlinkButton from "@winui/components/WinHyperlinkButton.vue";
import WinSelectorBar from "@winui/components/WinSelectorBar.vue";
import WinContentDialog from "@winui/components/WinContentDialog.vue";
import WinProgressRing from "@winui/components/WinProgressRing.vue";
import AppIcon from "@/components/AppIcon.vue";
import { i18nKey, type I18n } from "@winui/components/i18n/index";

const i18n = inject<I18n>(i18nKey)!;

interface GitFile {
  path: string;
  x: string;
  y: string;
  staged: boolean;
}

interface GitStatus {
  branch: string;
  ahead: number;
  behind: number;
  files: GitFile[];
  clean: boolean;
}

interface GitCommit {
  hash: string;
  short_hash: string;
  author: string;
  date: string;
  message: string;
}

interface GhAuthState {
  gh_installed: boolean;
  logged_in: boolean;
  user: string;
  host: string;
}

const hasTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const checking = ref(true);
const gitInstalled = ref(false);
const ghInstalled = ref(false);
const installing = ref(false);
const installError = ref("");
const installLog = ref("");

const authState = ref<GhAuthState | null>(null);

const repoPath = ref("");
const repoError = ref("");
const status = ref<GitStatus | null>(null);
const commits = ref<GitCommit[]>([]);
const loading = ref(false);
const busy = ref(false);
const commitMsg = ref("");
const notice = ref("");

const stagedFiles = computed(
  () => status.value?.files.filter((f) => f.staged) ?? []
);
const unstagedFiles = computed(
  () => status.value?.files.filter((f) => !f.staged && f.x !== "?") ?? []
);
const untrackedFiles = computed(
  () => status.value?.files.filter((f) => f.x === "?") ?? []
);
const canCommit = computed(
  () =>
    stagedFiles.value.length > 0 &&
    commitMsg.value.trim().length > 0 &&
    !busy.value
);

const cloneUrl = ref("");
const cloneTarget = ref("");
const cloning = ref(false);

const branches = ref<string[]>([]);
const selectedBranch = ref("");

const gitTab = ref("changes");
const gitTabItems = computed(() => [
  { Text: i18n.t("git.tabChanges"), Tag: "changes" },
  { Text: i18n.t("git.tabHistory"), Tag: "history" },
]);

function onGitTabChanged(sender: any) {
  const selected = sender?.SelectedItem;
  const index = Math.max(0, sender?.Items?.indexOf(selected) ?? 0);
  gitTab.value = sender?.Items?.[index]?.Tag ?? "changes";
}

// 登录流程状态
const needLoginDialog = ref(false);
const loginWizardOpen = ref(false);
const loginStep = ref(0);
const loginLogs = ref<string[]>([]);
const loginError = ref("");
const loginUserName = ref("");
const loginUserEmail = ref("");
const logoutBusy = ref(false);

// 集成终端面板
const terminalOutput = ref("");
const terminalInput = ref("");
const terminalExited = ref(false);
const terminalRef = ref<HTMLDivElement | null>(null);
let ptyUnlisten: UnlistenFn | null = null;

function appendLog(line: string) {
  loginLogs.value.push(`[${new Date().toLocaleTimeString()}] ${line}`);
}

function appendTerminal(text: string) {
  terminalOutput.value += text;
  // 检测进程退出标记
  if (text.includes("[gh 进程已退出")) {
    terminalExited.value = true;
  }
  nextTick(() => {
    const el = terminalRef.value;
    if (el) el.scrollTop = el.scrollHeight;
  });
}

async function sendTerminalInput() {
  const text = terminalInput.value;
  if (!text && !terminalExited.value) return;
  // 进程已退出时按回车也只发送换行
  const payload = text + "\r";
  try {
    await invoke("gh_login_input", { text: payload });
    terminalInput.value = "";
  } catch (e) {
    loginError.value = String(e);
  }
}

async function sendEnter() {
  try {
    await invoke("gh_login_input", { text: "\r" });
  } catch (e) {
    loginError.value = String(e);
  }
}

async function terminatePty() {
  try {
    await invoke("gh_login_terminate");
  } catch {
    // ignore
  }
}

async function browseFolder() {
  try {
    const p = await invoke<string | null>("pick_folder");
    if (p) repoPath.value = p;
  } catch (e) {
    console.error(e);
  }
}

async function browseCloneTarget() {
  try {
    const p = await invoke<string | null>("pick_folder");
    if (p) cloneTarget.value = p;
  } catch (e) {
    console.error(e);
  }
}

async function cloneRepo() {
  if (!cloneUrl.value || !cloneTarget.value) return;
  cloning.value = true;
  notice.value = "";
  try {
    const dest = await invoke<string>("git_clone", {
      url: cloneUrl.value,
      targetDir: cloneTarget.value,
    });
    repoPath.value = dest;
    notice.value = "cloned";
    await detectRepo();
  } catch (e) {
    notice.value = "";
    repoError.value = String(e);
  } finally {
    cloning.value = false;
  }
}

async function fetchRepo() {
  if (!repoPath.value) return;
  busy.value = true;
  notice.value = "";
  try {
    await invoke("git_fetch", { repo: repoPath.value });
    await refreshAll();
  } catch (e) {
    repoError.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function pullRepo() {
  if (!repoPath.value) return;
  busy.value = true;
  notice.value = "";
  try {
    await invoke("git_pull", { repo: repoPath.value });
    await refreshAll();
    notice.value = "pulled";
  } catch (e) {
    repoError.value = String(e);
  } finally {
    busy.value = false;
  }
}

const STATUS_KEYS: Record<string, string> = {
  M: "git.fileM",
  A: "git.fileA",
  D: "git.fileD",
  R: "git.fileR",
  C: "git.fileM",
  U: "git.fileU",
  "?": "git.fileU",
};

function statusText(f: GitFile): string {
  const code = f.x === "?" ? "?" : f.y !== " " ? f.y : f.x;
  return i18n.t(STATUS_KEYS[code] ?? "git.fileM");
}

async function detectRepo() {
  repoError.value = "";
  notice.value = "";
  if (!repoPath.value.trim()) return;
  try {
    const root = await invoke<string | null>("git_repo_root", {
      path: repoPath.value,
    });
    if (root) {
      repoPath.value = root;
      await refreshAll();
      await refreshBranches();
      await refreshAuth();
      await checkLoginNeeded();
    } else {
      repoError.value = "norepo";
      status.value = null;
      commits.value = [];
      branches.value = [];
    }
  } catch (e) {
    repoError.value = String(e);
  }
}

async function refreshAuth() {
  try {
    authState.value = await invoke<GhAuthState>("gh_auth_state");
  } catch {
    authState.value = null;
  }
}

async function checkLoginNeeded() {
  if (!ghInstalled.value) return;
  if (authState.value?.logged_in) return;
  if (loginWizardOpen.value) return;
  if (needLoginDialog.value) return;
  needLoginDialog.value = true;
}

async function refreshBranches() {
  if (!repoPath.value) return;
  try {
    branches.value = await invoke<string[]>("git_branches", {
      repo: repoPath.value,
    });
    if (status.value?.branch && branches.value.includes(status.value.branch)) {
      selectedBranch.value = status.value.branch;
    } else if (branches.value.length) {
      selectedBranch.value = branches.value[0];
    }
  } catch {
    branches.value = [];
  }
}

function onBranchChange(e: Event) {
  selectedBranch.value = (e.target as HTMLSelectElement).value;
}

async function refreshAll() {
  if (!repoPath.value) return;
  loading.value = true;
  repoError.value = "";
  try {
    status.value = await invoke<GitStatus>("git_status", {
      repo: repoPath.value,
    });
    commits.value = await invoke<GitCommit[]>("git_log", {
      repo: repoPath.value,
      limit: 50,
    });
  } catch (e) {
    repoError.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function stage(paths: string[]) {
  try {
    await invoke("git_add", { repo: repoPath.value, paths });
    await refreshAll();
  } catch (e) {
    repoError.value = String(e);
  }
}

async function unstage(paths: string[]) {
  try {
    await invoke("git_unstage", { repo: repoPath.value, paths });
    await refreshAll();
  } catch (e) {
    repoError.value = String(e);
  }
}

async function commit() {
  busy.value = true;
  repoError.value = "";
  try {
    await invoke("git_commit", {
      repo: repoPath.value,
      message: commitMsg.value,
      branch: selectedBranch.value || undefined,
    });
    commitMsg.value = "";
    notice.value = "ok";
    await refreshAll();
    await refreshBranches();
  } catch (e) {
    repoError.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function push() {
  busy.value = true;
  repoError.value = "";
  try {
    await invoke("git_push", {
      repo: repoPath.value,
      branch: selectedBranch.value || undefined,
    });
    notice.value = "pushed";
    await refreshAll();
  } catch (e) {
    repoError.value = String(e);
  } finally {
    busy.value = false;
  }
}

async function installGitAndGhNow() {
  installing.value = true;
  installError.value = "";
  installLog.value = "";
  try {
    const out = await invoke<string>("install_git_and_gh");
    installLog.value = out;
    // 重新检测安装状态
    gitInstalled.value = await invoke<boolean>("check_git");
    authState.value = await invoke<GhAuthState>("gh_auth_state");
    ghInstalled.value = authState.value?.gh_installed ?? false;
    if (gitInstalled.value && ghInstalled.value) {
      repoPath.value = await invoke<string>("git_default_dir");
      await detectRepo();
    }
  } catch (e) {
    installError.value = String(e);
  } finally {
    installing.value = false;
  }
}

function openGitSite() {
  window.open("https://git-scm.com/downloads", "_blank", "noopener,noreferrer");
}

function openGhSite() {
  window.open("https://cli.github.com/", "_blank", "noopener,noreferrer");
}

async function logoutGithub() {
  logoutBusy.value = true;
  notice.value = "";
  try {
    await invoke<string>("gh_logout");
    await refreshAuth();
    notice.value = "loggedOut";
  } catch (e) {
    repoError.value = String(e);
  } finally {
    logoutBusy.value = false;
  }
}

async function startLoginWizard() {
  needLoginDialog.value = false;
  loginWizardOpen.value = true;
  loginStep.value = 0;
  loginLogs.value = [];
  loginError.value = "";
  loginUserName.value = "";
  loginUserEmail.value = "";
  terminalOutput.value = "";
  terminalInput.value = "";
  terminalExited.value = false;

  // 启动事件监听
  if (ptyUnlisten) {
    ptyUnlisten();
    ptyUnlisten = null;
  }
  try {
    ptyUnlisten = await listen<string>("gh-login-output", (e) => {
      appendTerminal(e.payload);
    });
  } catch (e) {
    loginError.value = String(e);
    return;
  }

  appendLog(i18n.t("git.wizStart"));
  appendLog(i18n.t("git.wizStep1"));
  loginStep.value = 1;
  try {
    await invoke("gh_login_interactive");
    appendLog(i18n.t("git.wizLoginWindowOpened"));
    loginStep.value = 2;
    appendLog(i18n.t("git.wizWaitingLogin"));
    // 等待终端退出，由 PTY 输出事件触发
    await waitForTerminalExit(300);
    // 退出后检测登录状态
    await refreshAuth();
    if (authState.value?.logged_in) {
      const user = authState.value.user || "unknown";
      appendLog(i18n.t("git.wizLoginSuccess", { user }));
      loginStep.value = 3;
      appendLog(i18n.t("git.wizStep3"));
      if (repoPath.value) {
        try {
          await invoke("gh_setup_git", { repo: repoPath.value });
          appendLog(i18n.t("git.wizSetupGitOk"));
        } catch (e) {
          appendLog(i18n.t("git.wizSetupGitFail", { err: String(e) }));
        }
      } else {
        appendLog(i18n.t("git.wizSetupGitSkip"));
      }
      loginStep.value = 4;
      appendLog(i18n.t("git.wizStep4"));
      try {
        const cfg = await invoke<[string, string]>("git_get_user_config");
        loginUserName.value = cfg[0] || "";
        loginUserEmail.value = cfg[1] || "";
      } catch {
        // ignore
      }
    } else {
      appendLog(i18n.t("git.wizError", { err: "未检测到登录状态" }));
    }
  } catch (e) {
    loginError.value = String(e);
    appendLog(i18n.t("git.wizError", { err: String(e) }));
  }
}

function waitForTerminalExit(timeoutSecs: number): Promise<void> {
  const start = Date.now();
  return new Promise((resolve) => {
    const tick = () => {
      if (terminalExited.value) {
        resolve();
        return;
      }
      if (Date.now() - start > timeoutSecs * 1000) {
        resolve();
        return;
      }
      setTimeout(tick, 500);
    };
    tick();
  });
}

async function saveGitConfig() {
  if (!loginUserName.value.trim() || !loginUserEmail.value.trim()) {
    loginError.value = i18n.t("git.wizConfigEmpty");
    return;
  }
  loginError.value = "";
  try {
    await invoke("git_config_user", {
      name: loginUserName.value,
      email: loginUserEmail.value,
    });
    appendLog(i18n.t("git.wizConfigSaved", { name: loginUserName.value, email: loginUserEmail.value }));
    loginStep.value = 5;
    appendLog(i18n.t("git.wizDone"));
    await refreshAuth();
    setTimeout(() => {
      loginWizardOpen.value = false;
    }, 2000);
  } catch (e) {
    loginError.value = String(e);
    appendLog(i18n.t("git.wizError", { err: String(e) }));
  }
}

function closeLoginWizard() {
  loginWizardOpen.value = false;
  // 关闭时终止未完成的 PTY 会话
  if (!terminalExited.value) {
    terminatePty();
  }
  if (ptyUnlisten) {
    ptyUnlisten();
    ptyUnlisten = null;
  }
  refreshAuth();
}

onMounted(async () => {
  if (!hasTauri) {
    checking.value = false;
    return;
  }
  try {
    gitInstalled.value = await invoke<boolean>("check_git");
    authState.value = await invoke<GhAuthState>("gh_auth_state");
    ghInstalled.value = authState.value?.gh_installed ?? false;
    if (gitInstalled.value && ghInstalled.value) {
      repoPath.value = await invoke<string>("git_default_dir");
      await detectRepo();
    }
  } catch (e) {
    console.error(e);
  } finally {
    checking.value = false;
  }
});

onBeforeUnmount(() => {
  if (ptyUnlisten) {
    ptyUnlisten();
    ptyUnlisten = null;
  }
  if (!terminalExited.value) {
    terminatePty();
  }
});
</script>

<template>
  <PageShell title-key="git.title" subtitle-key="git.subtitle">
    <div v-if="checking" class="git-checking">
      <WinProgressRing
        :IsActive="true"
        :IsIndeterminate="true"
        :Width="36"
        :Height="36"
      />
    </div>

    <div v-else-if="!gitInstalled || !ghInstalled" class="git-card git-install-card">
      <div class="git-install-icon">
        <AppIcon name="git" :size="32" />
      </div>
      <div class="git-install-body">
        <WinTextBlock :Text="i18n.t('git.toolsNotInstalled')" Style="font-size:18px;font-weight:600" />
        <WinTextBlock :Text="i18n.t('git.toolsInstallHint')" Style="font-size:13px;opacity:.75" Foreground="secondary" />
        <div class="git-install-actions">
          <WinButton
            Style="AccentButtonStyle"
            :Content="installing ? i18n.t('git.installing') : i18n.t('git.installBoth')"
            :IsEnabled="!installing"
            @click="installGitAndGhNow"
          />
          <WinHyperlinkButton
            NavigateUri="https://git-scm.com/downloads"
            TargetName="_blank"
            :Content="i18n.t('git.downloadGit')"
          />
          <WinHyperlinkButton
            NavigateUri="https://cli.github.com/"
            TargetName="_blank"
            :Content="i18n.t('git.downloadGh')"
          />
        </div>
        <WinTextBlock
          v-if="installing"
          :Text="i18n.t('git.installingHint')"
          Style="font-size:12px;opacity:.6"
          Foreground="secondary"
        />
        <WinTextBlock
          v-if="installError"
          :Text="`${i18n.t('git.installFailed')}${installError}`"
          Style="font-size:12px;color:var(--system-error, #c42b1c)"
        />
      </div>
    </div>

    <template v-else>
      <div class="git-toolbar">
        <WinTextBox
          :Text="repoPath"
          :PlaceholderText="i18n.t('git.repoPath')"
          @update:Text="(v: string) => (repoPath = v)"
          Style="flex:1;min-width:220px"
        />
        <WinButton
          :Content="i18n.t('git.browse')"
          @click="browseFolder"
          :IsEnabled="!loading"
        />
        <WinButton
          :Content="i18n.t('git.detect')"
          @click="detectRepo"
          :IsEnabled="!loading"
        />
        <WinButton
          :Content="i18n.t('git.refresh')"
          @click="refreshAll"
          :IsEnabled="!loading && !repoError"
        />
      </div>

      <div class="git-clone-box">
        <WinTextBox
          :Text="cloneUrl"
          :PlaceholderText="i18n.t('git.cloneUrlPlaceholder')"
          @update:Text="(v: string) => (cloneUrl = v)"
          Style="flex:1;min-width:200px"
        />
        <WinTextBox
          :Text="cloneTarget"
          :PlaceholderText="i18n.t('git.cloneTargetPlaceholder')"
          @update:Text="(v: string) => (cloneTarget = v)"
          Style="flex:1;min-width:160px"
        />
        <WinButton
          :Content="i18n.t('git.browse')"
          @click="browseCloneTarget"
          :IsEnabled="!cloning"
        />
        <WinButton
          :Content="i18n.t('git.clone')"
          Style="AccentButtonStyle"
          @click="cloneRepo"
          :IsEnabled="!cloning && !!cloneUrl && !!cloneTarget"
        />
      </div>

      <div class="git-commit-box">
        <WinTextBox
          :Text="commitMsg"
          :PlaceholderText="i18n.t('git.commitPlaceholder')"
          @update:Text="(v: string) => (commitMsg = v)"
          Style="flex:1;min-width:200px"
        />
        <WinButton
          :Content="i18n.t('git.commit')"
          Style="AccentButtonStyle"
          @click="commit"
          :IsEnabled="canCommit"
        />
      </div>

      <div v-if="repoError === 'norepo'" class="git-card git-warn">
        <WinTextBlock :Text="i18n.t('git.noRepo')" Style="color:var(--system-error, #c42b1c)" />
      </div>
      <div v-else-if="repoError" class="git-card git-warn">
        <WinTextBlock :Text="repoError" Style="color:var(--system-error, #c42b1c)" />
      </div>
      <div v-if="notice === 'ok'" class="git-card git-ok">
        <WinTextBlock :Text="i18n.t('git.commitSuccess')" Style="color:var(--accent, #005fb8)" />
      </div>
      <div v-if="notice === 'pushed'" class="git-card git-ok">
        <WinTextBlock :Text="i18n.t('git.pushSuccess')" Style="color:var(--accent, #005fb8)" />
      </div>
      <div v-if="notice === 'cloned'" class="git-card git-ok">
        <WinTextBlock :Text="i18n.t('git.cloneSuccess')" Style="color:var(--accent, #005fb8)" />
      </div>
      <div v-if="notice === 'pulled'" class="git-card git-ok">
        <WinTextBlock :Text="i18n.t('git.pullSuccess')" Style="color:var(--accent, #005fb8)" />
      </div>
      <div v-if="notice === 'loggedOut'" class="git-card git-ok">
        <WinTextBlock :Text="i18n.t('git.logoutSuccess')" Style="color:var(--accent, #005fb8)" />
      </div>

      <template v-if="status">
        <WinSelectorBar
          class="git-selector"
          :Items="gitTabItems"
          :SelectedItem="gitTabItems[gitTab === 'changes' ? 0 : 1]"
          @SelectionChanged="onGitTabChanged"
        />

        <section v-if="gitTab === 'changes'" class="git-card">
          <div class="git-branch-row">
            <AppIcon name="git" :size="20" />
            <WinTextBlock :Text="status.branch || '—'" Style="font-size:16px;font-weight:600" />
            <WinTextBlock
              v-if="authState?.logged_in && authState?.user"
              :Text="i18n.t('git.loggedAs', { user: authState.user, host: authState.host || 'github.com' })"
              Style="font-size:12px"
              Foreground="secondary"
            />
            <WinTextBlock
              v-if="status.ahead > 0"
              :Text="i18n.t('git.ahead', { n: status.ahead })"
              Style="font-size:12px"
              Foreground="secondary"
            />
            <WinTextBlock
              v-if="status.behind > 0"
              :Text="i18n.t('git.behind', { n: status.behind })"
              Style="font-size:12px"
              Foreground="secondary"
            />
            <div class="git-branch-actions" v-if="branches.length">
              <select class="git-branch-select" :value="selectedBranch" @change="onBranchChange">
                <option v-for="b in branches" :key="b" :value="b">{{ b }}</option>
              </select>
            </div>
          </div>

          <template v-if="status.clean">
            <WinTextBlock :Text="i18n.t('git.clean')" Style="opacity:.6;padding:8px 0" />
          </template>

          <template v-else>
            <div v-if="stagedFiles.length" class="git-group">
              <div class="git-group-header">
                <WinTextBlock :Text="i18n.t('git.staged')" Style="font-size:13px;font-weight:600" />
                <WinTextBlock :Text="String(stagedFiles.length)" Style="font-size:12px" Foreground="secondary" />
                <WinButton
                  class="git-group-btn"
                  :Content="i18n.t('git.unstageAll')"
                  @click="unstage(stagedFiles.map((f) => f.path))"
                  :IsEnabled="!busy"
                />
              </div>
              <div v-for="f in stagedFiles" :key="f.path" class="git-file">
                <span class="git-file-badge staged">{{ statusText(f) }}</span>
                <span class="git-file-path">{{ f.path }}</span>
                <WinButton
                  class="git-file-btn"
                  :Content="i18n.t('git.unstage')"
                  @click="unstage([f.path])"
                  :IsEnabled="!busy"
                />
              </div>
            </div>

            <div v-if="unstagedFiles.length" class="git-group">
              <div class="git-group-header">
                <WinTextBlock :Text="i18n.t('git.changes')" Style="font-size:13px;font-weight:600" />
                <WinTextBlock :Text="String(unstagedFiles.length)" Style="font-size:12px" Foreground="secondary" />
                <WinButton
                  class="git-group-btn"
                  :Content="i18n.t('git.stageAll')"
                  @click="stage(unstagedFiles.map((f) => f.path))"
                  :IsEnabled="!busy"
                />
              </div>
              <div v-for="f in unstagedFiles" :key="f.path" class="git-file">
                <span class="git-file-badge">{{ statusText(f) }}</span>
                <span class="git-file-path">{{ f.path }}</span>
                <WinButton
                  class="git-file-btn"
                  :Content="i18n.t('git.stage')"
                  @click="stage([f.path])"
                  :IsEnabled="!busy"
                />
              </div>
            </div>

            <div v-if="untrackedFiles.length" class="git-group">
              <div class="git-group-header">
                <WinTextBlock :Text="i18n.t('git.untracked')" Style="font-size:13px;font-weight:600" />
                <WinTextBlock :Text="String(untrackedFiles.length)" Style="font-size:12px" Foreground="secondary" />
                <WinButton
                  class="git-group-btn"
                  :Content="i18n.t('git.stageAll')"
                  @click="stage(untrackedFiles.map((f) => f.path))"
                  :IsEnabled="!busy"
                />
              </div>
              <div v-for="f in untrackedFiles" :key="f.path" class="git-file">
                <span class="git-file-badge untracked">{{ statusText(f) }}</span>
                <span class="git-file-path">{{ f.path }}</span>
                <WinButton
                  class="git-file-btn"
                  :Content="i18n.t('git.stage')"
                  @click="stage([f.path])"
                  :IsEnabled="!busy"
                />
              </div>
            </div>
          </template>

          <div class="git-remote-actions">
            <WinButton
              :Content="i18n.t('git.push')"
              Style="AccentButtonStyle"
              @click="push"
              :IsEnabled="!busy"
            />
            <WinButton
              v-if="authState?.logged_in"
              :Content="logoutBusy ? i18n.t('git.loggingOut') : i18n.t('git.switchAccount')"
              @click="startLoginWizard"
              :IsEnabled="!busy && !logoutBusy"
            />
            <WinButton
              v-if="authState?.logged_in"
              :Content="logoutBusy ? i18n.t('git.loggingOut') : i18n.t('git.logout')"
              @click="logoutGithub"
              :IsEnabled="!busy && !logoutBusy"
            />
            <WinButton
              v-else
              :Content="i18n.t('git.relogin')"
              @click="startLoginWizard"
              :IsEnabled="!busy"
            />
          </div>
        </section>

        <section v-if="gitTab === 'history'" class="git-card">
          <div class="git-group-header">
            <WinTextBlock :Text="i18n.t('git.history')" Style="font-size:14px;font-weight:600" />
            <div class="git-history-actions">
              <WinButton
                :Content="i18n.t('git.fetch')"
                @click="fetchRepo"
                :IsEnabled="!busy"
              />
              <WinButton
                :Content="i18n.t('git.pull')"
                Style="AccentButtonStyle"
                @click="pullRepo"
                :IsEnabled="!busy"
              />
            </div>
          </div>
          <div v-if="!commits.length" class="git-hint">
            <WinTextBlock :Text="i18n.t('git.empty')" Style="opacity:.6" />
          </div>
          <div v-for="c in commits" :key="c.hash" class="git-commit-row">
            <span class="git-commit-hash">{{ c.short_hash }}</span>
            <span class="git-commit-msg">{{ c.message }}</span>
            <span class="git-commit-meta">{{ c.author }} · {{ c.date }}</span>
          </div>
        </section>
      </template>
    </template>

    <WinContentDialog
      v-model:IsOpen="needLoginDialog"
      :Title="i18n.t('git.needLoginTitle')"
      :Content="i18n.t('git.needLoginContent')"
      :PrimaryButtonText="i18n.t('git.needLoginConfirm')"
      :CloseButtonText="i18n.t('git.needLoginCancel')"
      DefaultButton="Primary"
      @PrimaryButtonClick="startLoginWizard"
    />

    <WinContentDialog
      v-model:IsOpen="loginWizardOpen"
      :Title="i18n.t('git.wizardTitle')"
      :PrimaryButtonText="loginStep >= 4 ? i18n.t('git.wizSave') : ''"
      :CloseButtonText="loginStep >= 5 ? i18n.t('git.wizClose') : i18n.t('git.wizCancel')"
      :IsPrimaryButtonEnabled="loginStep >= 4 && !!loginUserName.trim() && !!loginUserEmail.trim()"
      @PrimaryButtonClick="saveGitConfig"
      @CloseButtonClick="closeLoginWizard"
    >
      <div class="wiz-container">
        <div class="wiz-steps">
          <div class="wiz-step" :class="{ active: loginStep === 1, done: loginStep > 1 }">1. {{ i18n.t('git.wizStep1Short') }}</div>
          <div class="wiz-step" :class="{ active: loginStep === 2, done: loginStep > 2 }">2. {{ i18n.t('git.wizStep2Short') }}</div>
          <div class="wiz-step" :class="{ active: loginStep === 3, done: loginStep > 3 }">3. {{ i18n.t('git.wizStep3Short') }}</div>
          <div class="wiz-step" :class="{ active: loginStep === 4, done: loginStep > 4 }">4. {{ i18n.t('git.wizStep4Short') }}</div>
        </div>

        <div class="wiz-terminal-panel">
          <div class="wiz-terminal-header">
            <span class="wiz-terminal-title">{{ i18n.t('git.wizTerminal') }}</span>
            <div class="wiz-terminal-actions">
              <WinButton
                :Content="i18n.t('git.wizEnter')"
                @click="sendEnter"
                :IsEnabled="!terminalExited"
                Style="font-size:12px"
              />
              <WinButton
                :Content="i18n.t('git.wizAbort')"
                @click="terminatePty"
                :IsEnabled="!terminalExited"
                Style="font-size:12px"
              />
            </div>
          </div>
          <div ref="terminalRef" class="wiz-terminal-output">
            <pre>{{ terminalOutput || i18n.t('git.wizTerminalEmpty') }}</pre>
          </div>
          <div class="wiz-terminal-input-row">
            <input
              v-model="terminalInput"
              class="wiz-terminal-input"
              :placeholder="i18n.t('git.wizTerminalInputHint')"
              :disabled="terminalExited"
              @keydown.enter.prevent="sendTerminalInput"
            />
            <WinButton
              :Content="i18n.t('git.wizSend')"
              @click="sendTerminalInput"
              :IsEnabled="!terminalExited"
              Style="font-size:12px"
            />
          </div>
          <div v-if="loginLogs.length" class="wiz-logs">
            <div v-for="(line, i) in loginLogs" :key="i" class="wiz-log-line">{{ line }}</div>
            <div v-if="loginError" class="wiz-log-line wiz-log-error">{{ loginError }}</div>
          </div>
        </div>

        <div v-if="loginStep === 4" class="wiz-config-form">
          <div class="wiz-config-row">
            <label>{{ i18n.t('git.wizName') }}</label>
            <input
              v-model="loginUserName"
              class="wiz-config-input"
              :placeholder="i18n.t('git.wizNamePlaceholder')"
            />
          </div>
          <div class="wiz-config-row">
            <label>{{ i18n.t('git.wizEmail') }}</label>
            <input
              v-model="loginUserEmail"
              class="wiz-config-input"
              :placeholder="i18n.t('git.wizEmailPlaceholder')"
            />
          </div>
        </div>

        <div v-if="loginStep === 5" class="wiz-done">
          <WinTextBlock :Text="i18n.t('git.wizDone')" Style="color:var(--accent, #005fb8)" />
        </div>
      </div>
    </WinContentDialog>
  </PageShell>
</template>

<style scoped>
.git-hint {
  padding: 12px 0;
}

.git-checking {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 48px 0;
}

.git-toolbar {
  position: sticky;
  top: 0;
  z-index: 10;
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  margin: -4px -8px 4px;
  padding: 8px 8px;
  border-radius: 8px;
  background: var(--SolidBackgroundFillColorBaseBrush, rgba(243, 243, 243, 0.85));
  backdrop-filter: blur(12px);
}

html.theme-dark .git-toolbar {
  background: var(--SolidBackgroundFillColorBaseBrush, rgba(32, 32, 32, 0.85));
}

.git-selector {
  align-self: center;
}

.git-card {
  border-radius: 8px;
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.08));
  background: var(--LayerFillColorDefaultBrush, rgba(255, 255, 255, 0.5));
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

html.theme-dark .git-card {
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.07));
  background: var(--LayerFillColorDefaultBrush, rgba(255, 255, 255, 0.03));
}

.git-warn {
  border-color: var(--system-error, rgba(196, 43, 28, 0.4));
}

.git-install-card {
  flex-direction: row;
  align-items: center;
  gap: 20px;
  padding: 28px;
}

.git-install-icon {
  width: 64px;
  height: 64px;
  border-radius: 12px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--AccentButtonBackground, #005fb8);
  background: color-mix(in srgb, var(--AccentButtonBackground, #005fb8) 14%, transparent);
  flex-shrink: 0;
}

html.theme-dark .git-install-icon {
  color: #4cc2ff;
  background: color-mix(in srgb, #4cc2ff 16%, transparent);
}

.git-install-body {
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-items: flex-start;
}

.git-install-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.git-branch-row {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--AccentButtonBackground, #005fb8);
}

html.theme-dark .git-branch-row {
  color: #4cc2ff;
}

.git-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.git-group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
}

.git-group-btn {
  margin-left: auto;
}

.git-file {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 8px;
  border-radius: 6px;
}

.git-file:hover {
  background: var(--ControlFillColorSecondaryBrush, rgba(0, 0, 0, 0.04));
}

html.theme-dark .git-file:hover {
  background: var(--ControlFillColorSecondaryBrush, rgba(255, 255, 255, 0.05));
}

.git-file-badge {
  flex-shrink: 0;
  min-width: 48px;
  text-align: center;
  font-size: 11px;
  line-height: 18px;
  padding: 0 8px;
  border-radius: 9px;
  color: var(--AccentButtonBackground, #005fb8);
  background: color-mix(in srgb, var(--AccentButtonBackground, #005fb8) 12%, transparent);
}

html.theme-dark .git-file-badge {
  color: #4cc2ff;
  background: color-mix(in srgb, #4cc2ff 14%, transparent);
}

.git-file-badge.staged {
  color: #6ccb5f;
  background: color-mix(in srgb, #6ccb5f 16%, transparent);
}

.git-file-badge.untracked {
  color: #f59e0b;
  background: color-mix(in srgb, #f59e0b 16%, transparent);
}

.git-file-path {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  font-family: "Cascadia Code", "Consolas", monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  direction: rtl;
  text-align: left;
}

.git-file-btn {
  flex-shrink: 0;
}

.git-commit-box {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  margin-top: 8px;
}

.git-clone-box {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  margin-top: 8px;
  padding: 10px 12px;
  border-radius: 8px;
  background: var(--CardBackgroundFillColorDefaultBrush, rgba(255, 255, 255, 0.7));
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.06));
}

html.theme-dark .git-clone-box {
  background: var(--CardBackgroundFillColorDefaultBrush, rgba(32, 32, 32, 0.6));
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.07));
}

.git-history-actions {
  display: flex;
  gap: 8px;
  margin-left: auto;
}

.git-branch-actions {
  margin-left: auto;
}

.git-branch-select {
  padding: 4px 8px;
  border-radius: 6px;
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.12));
  background: var(--SolidBackgroundFillColorBaseBrush, rgba(255, 255, 255, 0.6));
  color: inherit;
  font-size: 13px;
  min-width: 140px;
  cursor: pointer;
}

html.theme-dark .git-branch-select {
  background: var(--SolidBackgroundFillColorBaseBrush, rgba(32, 32, 32, 0.6));
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.1));
}

.git-remote-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.06));
}

html.theme-dark .git-remote-actions {
  border-top-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.07));
}

.git-commit-row {
  display: flex;
  align-items: baseline;
  gap: 10px;
  padding: 4px 0;
  font-size: 13px;
}

.git-commit-hash {
  flex-shrink: 0;
  font-family: "Cascadia Code", "Consolas", monospace;
  color: var(--AccentButtonBackground, #005fb8);
}

html.theme-dark .git-commit-hash {
  color: #4cc2ff;
}

.git-commit-msg {
  flex: 1;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.git-commit-meta {
  flex-shrink: 0;
  font-size: 12px;
  opacity: 0.65;
}

.wiz-container {
  display: flex;
  flex-direction: column;
  gap: 12px;
  width: 100%;
  min-width: 480px;
  max-width: 640px;
}

.wiz-steps {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.wiz-step {
  flex: 1;
  min-width: 100px;
  padding: 6px 10px;
  border-radius: 6px;
  background: var(--ControlFillColorSecondaryBrush, rgba(0, 0, 0, 0.04));
  font-size: 12px;
  color: var(--TextFillColorSecondaryBrush, rgba(0, 0, 0, 0.6));
  border: 1px solid transparent;
}

html.theme-dark .wiz-step {
  background: var(--ControlFillColorSecondaryBrush, rgba(255, 255, 255, 0.05));
  color: var(--TextFillColorSecondaryBrush, rgba(255, 255, 255, 0.6));
}

.wiz-step.active {
  border-color: var(--AccentButtonBackground, #005fb8);
  color: var(--AccentButtonBackground, #005fb8);
  font-weight: 600;
}

html.theme-dark .wiz-step.active {
  border-color: #4cc2ff;
  color: #4cc2ff;
}

.wiz-step.done {
  color: #6ccb5f;
  background: color-mix(in srgb, #6ccb5f 12%, transparent);
}

.wiz-log-panel {
  background: rgba(0, 0, 0, 0.04);
  border-radius: 6px;
  padding: 10px 12px;
  max-height: 200px;
  overflow-y: auto;
  font-family: "Cascadia Code", "Consolas", monospace;
  font-size: 12px;
  line-height: 1.6;
}

html.theme-dark .wiz-log-panel {
  background: rgba(0, 0, 0, 0.3);
  color: rgba(255, 255, 255, 0.85);
}

.wiz-log-line {
  white-space: pre-wrap;
  word-break: break-word;
}

.wiz-log-error {
  color: var(--system-error, #c42b1c);
}

.wiz-terminal-panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.wiz-terminal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.wiz-terminal-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--TextFillColorSecondaryBrush, rgba(0, 0, 0, 0.7));
}

html.theme-dark .wiz-terminal-title {
  color: var(--TextFillColorSecondaryBrush, rgba(255, 255, 255, 0.7));
}

.wiz-terminal-actions {
  display: flex;
  gap: 6px;
}

.wiz-terminal-output {
  background: #0c0c0c;
  color: #e6e6e6;
  border-radius: 6px;
  padding: 10px 12px;
  height: 220px;
  max-height: 220px;
  overflow-y: auto;
  font-family: "Cascadia Code", "Consolas", monospace;
  font-size: 12px;
  line-height: 1.5;
}

.wiz-terminal-output pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: inherit;
  font-size: inherit;
}

.wiz-terminal-input-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.wiz-terminal-input {
  flex: 1;
  padding: 6px 10px;
  border-radius: 6px;
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.12));
  background: var(--SolidBackgroundFillColorBaseBrush, rgba(255, 255, 255, 0.6));
  color: inherit;
  font-size: 13px;
  font-family: "Cascadia Code", "Consolas", monospace;
}

html.theme-dark .wiz-terminal-input {
  background: var(--SolidBackgroundFillColorBaseBrush, rgba(32, 32, 32, 0.6));
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.1));
}

.wiz-terminal-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.wiz-logs {
  max-height: 120px;
  overflow-y: auto;
  padding: 8px 10px;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.04);
  font-family: "Cascadia Code", "Consolas", monospace;
  font-size: 11px;
  line-height: 1.5;
}

html.theme-dark .wiz-logs {
  background: rgba(0, 0, 0, 0.3);
  color: rgba(255, 255, 255, 0.85);
}

.wiz-config-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 10px 0;
}

.wiz-config-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.wiz-config-row label {
  width: 80px;
  font-size: 13px;
  flex-shrink: 0;
}

.wiz-config-input {
  flex: 1;
  padding: 6px 10px;
  border-radius: 6px;
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.12));
  background: var(--SolidBackgroundFillColorBaseBrush, rgba(255, 255, 255, 0.6));
  color: inherit;
  font-size: 13px;
}

html.theme-dark .wiz-config-input {
  background: var(--SolidBackgroundFillColorBaseBrush, rgba(32, 32, 32, 0.6));
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.1));
}

.wiz-done {
  padding: 8px 0;
}
</style>
