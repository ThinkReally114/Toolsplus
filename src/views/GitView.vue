<script setup lang="ts">
import { inject, ref, computed, onMounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import PageShell from "@/components/PageShell.vue";
import WinTextBlock from "@winui/components/WinTextBlock.vue";
import WinButton from "@winui/components/WinButton.vue";
import WinTextBox from "@winui/components/WinTextBox.vue";
import WinHyperlinkButton from "@winui/components/WinHyperlinkButton.vue";
import WinSelectorBar from "@winui/components/WinSelectorBar.vue";
import WinContentDialog from "@winui/components/WinContentDialog.vue";
import WinProgressRing from "@winui/components/WinProgressRing.vue";
import WinCheckBox from "@winui/components/WinCheckBox.vue";
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
  body: string;
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
const commitPushDialog = ref(false);
const commitPushMsg = ref("");
const commitPushBody = ref("");
const commitPushBusy = ref(false);
const commitPushError = ref("");
const commitPushOnly = ref(false);
const resultDialog = ref(false);
const resultDialogTitle = ref("");
const resultDialogContent = ref("");
const repoSettingsDialog = ref(false);
const repoSettingsPath = ref("");
const repoSettingsBusy = ref(false);
const repoSettingsError = ref("");
const repoSettingsNotGit = ref(false);
const repoSettingsInitBusy = ref(false);
const revertDialog = ref(false);
const revertTarget = ref<GitCommit | null>(null);
const revertBusy = ref(false);
const revertError = ref("");
const revertNoCommit = ref(false);
const expandedCommit = ref<string | null>(null);

function toggleCommitExpand(c: GitCommit) {
  if (expandedCommit.value === c.hash) {
    expandedCommit.value = null;
  } else {
    expandedCommit.value = c.hash;
  }
}
const welcomeBusy = ref(false);
const welcomeInitBusy = ref(false);
const welcomeError = ref("");

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
  () => stagedFiles.value.length > 0 && !busy.value
);
const canCommitPush = computed(
  () =>
    stagedFiles.value.length > 0 &&
    commitPushMsg.value.trim().length > 0 &&
    !commitPushBusy.value
);

const repoName = computed(() => {
  const p = repoPath.value.trim();
  if (!p) return "";
  const sep = p.includes("/") ? "/" : "\\";
  const parts = p.split(sep).filter(Boolean);
  return parts.length ? parts[parts.length - 1] : p;
});

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
const logoutConfirmDialog = ref(false);
const loginWizardOpen = ref(false);
const loginStep = ref(0);
const loginLogs = ref<string[]>([]);
const loginError = ref("");
const loginUserName = ref("");
const loginUserEmail = ref("");
const logoutBusy = ref(false);

function appendLog(line: string) {
  loginLogs.value.push(`[${new Date().toLocaleTimeString()}] ${line}`);
}

async function browseFolder() {
  try {
    const p = await invoke<string | null>("pick_folder");
    if (p) repoPath.value = p;
  } catch (e) {
    console.error(e);
  }
}

function openRepoSettings() {
  repoSettingsPath.value = repoPath.value;
  repoSettingsError.value = "";
  repoSettingsNotGit.value = false;
  repoSettingsInitBusy.value = false;
  repoSettingsDialog.value = true;
}

async function browseRepoSettings() {
  try {
    const p = await invoke<string | null>("pick_folder");
    if (p) repoSettingsPath.value = p;
  } catch (e) {
    console.error(e);
  }
}

async function confirmRepoSettings() {
  repoError.value = "";
  if (!repoSettingsPath.value.trim()) {
    repoSettingsError.value = i18n.t("git.repoSettingsEmpty");
    // WinContentDialog 在 PrimaryButtonClick 后会自动关闭，这里重新打开以显示错误
    await nextTick();
    repoSettingsDialog.value = true;
    return;
  }
  repoSettingsBusy.value = true;
  repoSettingsError.value = "";
  try {
    const root = await invoke<string | null>("git_repo_root", {
      path: repoSettingsPath.value,
    });
    if (!root) {
      repoSettingsError.value = i18n.t("git.noRepo");
      repoSettingsNotGit.value = true;
      repoSettingsBusy.value = false;
      // WinContentDialog 在 PrimaryButtonClick 后会自动关闭，这里重新打开以显示错误
      await nextTick();
      repoSettingsDialog.value = true;
      return;
    }
    repoPath.value = root;
    localStorage.setItem("git.repoPath", root);
    await refreshAll();
    await refreshBranches();
    await refreshAuth();
    await checkLoginNeeded();
    repoSettingsDialog.value = false;
  } catch (e) {
    repoSettingsError.value = String(e);
    await nextTick();
    repoSettingsDialog.value = true;
  } finally {
    repoSettingsBusy.value = false;
  }
}

async function initRepoAndEnterWizard() {
  if (!repoSettingsPath.value.trim()) {
    repoSettingsError.value = i18n.t("git.repoSettingsEmpty");
    await nextTick();
    repoSettingsDialog.value = true;
    return;
  }
  repoSettingsInitBusy.value = true;
  repoSettingsError.value = "";
  try {
    await invoke<string>("git_init", { path: repoSettingsPath.value });
    repoPath.value = repoSettingsPath.value;
    localStorage.setItem("git.repoPath", repoPath.value);
    repoSettingsDialog.value = false;
    await refreshAll();
    await refreshBranches();
    await refreshAuth();
    showResult(i18n.t("git.initSuccessTitle"), i18n.t("git.initSuccess"));
    await startLoginWizard();
  } catch (e) {
    repoSettingsError.value = String(e);
    await nextTick();
    repoSettingsDialog.value = true;
  } finally {
    repoSettingsInitBusy.value = false;
  }
}

async function resetRepoSettingsSelection() {
  repoSettingsNotGit.value = false;
  await confirmRepoSettings();
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
  repoSettingsError.value = "";
  try {
    const dest = await invoke<string>("git_clone", {
      url: cloneUrl.value,
      targetDir: cloneTarget.value,
    });
    repoPath.value = dest;
    repoSettingsPath.value = dest;
    cloneUrl.value = "";
    cloneTarget.value = "";
    await detectRepo();
    repoSettingsDialog.value = false;
    showResult(i18n.t("git.cloneSuccessTitle"), i18n.t("git.cloneSuccess"));
  } catch (e) {
    repoSettingsError.value = String(e);
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
    showResult(i18n.t("git.pullSuccessTitle"), i18n.t("git.pullSuccess"));
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

async function refreshAuth(retries = 3, delayMs = 800) {
  for (let attempt = 0; attempt < retries; attempt++) {
    try {
      const state = await invoke<GhAuthState>("gh_auth_state");
      authState.value = state;
      if (state.logged_in && state.user) return;
      if (!state.logged_in) return;
    } catch {
      authState.value = null;
    }
    await new Promise((r) => setTimeout(r, delayMs));
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

function showResult(title: string, content: string) {
  resultDialogTitle.value = title;
  resultDialogContent.value = content;
  resultDialog.value = true;
}

function openRevertDialog(c: GitCommit) {
  revertTarget.value = c;
  revertError.value = "";
  revertNoCommit.value = false;
  revertDialog.value = true;
}

async function confirmRevert() {
  if (!revertTarget.value) return;
  revertBusy.value = true;
  revertError.value = "";
  try {
    await invoke("git_revert", {
      repo: repoPath.value,
      hash: revertTarget.value.hash,
      noCommit: revertNoCommit.value,
    });
    await refreshAll();
    await refreshBranches();
    revertDialog.value = false;
    showResult(
      i18n.t("git.revertSuccessTitle"),
      i18n.t("git.revertSuccess", { hash: revertTarget.value.short_hash })
    );
  } catch (e) {
    revertError.value = String(e);
    await nextTick();
    revertDialog.value = true;
  } finally {
    revertBusy.value = false;
  }
}

function openCommitPushDialog() {
  commitPushMsg.value = commitMsg.value || "";
  commitPushBody.value = "";
  commitPushError.value = "";
  commitPushOnly.value = false;
  commitPushDialog.value = true;
}

async function confirmCommitPush() {
  if (!commitPushMsg.value.trim()) {
    commitPushError.value = i18n.t("git.commitMsgEmpty");
    await nextTick();
    commitPushDialog.value = true;
    return;
  }
  commitPushBusy.value = true;
  commitPushError.value = "";
  try {
    await invoke("git_commit", {
      repo: repoPath.value,
      message: commitPushMsg.value,
      body: commitPushBody.value.trim() || undefined,
      branch: selectedBranch.value || undefined,
    });
    commitMsg.value = "";
    await refreshAll();
    await refreshBranches();
    if (commitPushOnly.value) {
      commitPushDialog.value = false;
      showResult(i18n.t("git.commitSuccessTitle"), i18n.t("git.commitSuccess"));
    } else {
      await invoke("git_push", {
        repo: repoPath.value,
        branch: selectedBranch.value || undefined,
      });
      await refreshAll();
      commitPushDialog.value = false;
      showResult(i18n.t("git.pushSuccessTitle"), i18n.t("git.pushSuccess"));
    }
  } catch (e) {
    commitPushError.value = String(e);
    await nextTick();
    commitPushDialog.value = true;
  } finally {
    commitPushBusy.value = false;
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
    await refreshAll();
    showResult(i18n.t("git.pushSuccessTitle"), i18n.t("git.pushSuccess"));
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
  // 先弹二次确认对话框
  logoutConfirmDialog.value = true;
}

async function confirmLogout() {
  logoutConfirmDialog.value = false;
  logoutBusy.value = true;
  notice.value = "";
  try {
    await invoke<string>("gh_logout");
    await refreshAuth();
    // 退出后延迟再刷新，确保多账号场景下完全清空
    setTimeout(() => refreshAuth(), 1000);
    // 退出后弹出引导对话框：登录或跳过
    needLoginDialog.value = true;
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

  appendLog(i18n.t("git.wizStart"));
  appendLog(i18n.t("git.wizStep1"));
  loginStep.value = 1;
  try {
    await invoke("gh_login_interactive");
    appendLog(i18n.t("git.wizLoginWindowOpened"));
    loginStep.value = 2;
    appendLog(i18n.t("git.wizWaitingLogin"));
    const user = await invoke<string>("gh_wait_login", { timeoutSecs: 300 });
    if (!user) {
      throw new Error("未检测到登录用户名");
    }
    appendLog(i18n.t("git.wizLoginSuccess", { user }));
    // 登录成功后立即刷新一次 authState，确保 UI 不显示旧账号
    await refreshAuth();
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
  } catch (e) {
    loginError.value = String(e);
    appendLog(i18n.t("git.wizError", { err: String(e) }));
  }
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
    setTimeout(() => refreshAuth(), 1500);
    setTimeout(() => {
      loginWizardOpen.value = false;
    }, 2000);
  } catch (e) {
    loginError.value = String(e);
    appendLog(i18n.t("git.wizError", { err: String(e) }));
  }
}

function closeLoginWizard() {
  // 如果登录未完成（step < 5），先尝试取消 pw 登录窗口
  // step >= 5 表示已经完成配置，pw 窗口已自动关闭
  if (loginStep.value < 5) {
    invoke("gh_cancel_login").catch(() => {
      // 忽略错误，确保对话框一定能关闭
    });
  }
  loginWizardOpen.value = false;
  // 关闭时多次延迟刷新，确保 gh token 已完全切换
  refreshAuth();
  setTimeout(() => refreshAuth(), 1500);
  setTimeout(() => refreshAuth(), 3000);
}

async function welcomeCreateRepo() {
  if (welcomeInitBusy.value) return;
  welcomeError.value = "";
  try {
    const p = await invoke<string | null>("pick_folder");
    if (!p) return;
    welcomeInitBusy.value = true;
    await invoke<string>("git_init", { path: p });
    repoPath.value = p;
    await refreshAll();
    await refreshBranches();
    await refreshAuth();
    showResult(i18n.t("git.initSuccessTitle"), i18n.t("git.initSuccess"));
    await startLoginWizard();
  } catch (e) {
    welcomeError.value = String(e);
  } finally {
    welcomeInitBusy.value = false;
  }
}

async function welcomeEnterRepo() {
  if (welcomeBusy.value) return;
  welcomeError.value = "";
  try {
    const p = await invoke<string | null>("pick_folder");
    if (!p) return;
    welcomeBusy.value = true;
    repoSettingsPath.value = p;
    repoSettingsNotGit.value = false;
    repoSettingsError.value = "";
    await confirmRepoSettings();
    if (repoSettingsNotGit.value) {
      repoSettingsDialog.value = true;
    }
  } catch (e) {
    welcomeError.value = String(e);
  } finally {
    welcomeBusy.value = false;
  }
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
    const savedRepo = localStorage.getItem("git.repoPath");
    if (savedRepo) {
      const root = await invoke<string | null>("git_repo_root", { path: savedRepo });
      if (root) {
        repoPath.value = root;
        await refreshAll();
        await refreshBranches();
        await checkLoginNeeded();
      } else {
        localStorage.removeItem("git.repoPath");
      }
    }
  } catch (e) {
    console.error(e);
  } finally {
    checking.value = false;
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
      <div v-if="!repoPath" class="git-welcome">
        <div class="git-welcome-account">
          <WinTextBlock
            v-if="authState?.logged_in && authState?.user"
            :Text="i18n.t('git.loggedAs', { user: authState.user })"
            Style="font-size:13px"
            Foreground="secondary"
          />
          <WinTextBlock
            v-else
            :Text="i18n.t('git.notLoggedIn')"
            Style="font-size:13px"
            Foreground="secondary"
          />
          <WinButton
            :IsEnabled="!welcomeBusy && !welcomeInitBusy && !logoutBusy"
            @click="startLoginWizard"
            :Title="i18n.t('git.switchAccount')"
            Style="padding:4px;min-height:28px"
          >
            <AppIcon name="switchAccount" :size="16" />
          </WinButton>
          <WinButton
            v-if="authState?.logged_in"
            :IsEnabled="!welcomeBusy && !welcomeInitBusy && !logoutBusy"
            @click="logoutGithub"
            :Title="logoutBusy ? i18n.t('git.loggingOut') : i18n.t('git.logout')"
            Style="padding:4px;min-height:28px"
          >
            <AppIcon name="logout" :size="16" />
          </WinButton>
        </div>

        <div class="git-welcome-hero">
          <div class="git-welcome-icon">
            <AppIcon name="git" :size="40" />
          </div>
          <WinTextBlock
            :Text="i18n.t('git.welcomeTitle')"
            Style="font-size:22px;font-weight:600"
          />
          <WinTextBlock
            :Text="i18n.t('git.welcomeSubtitle')"
            Style="font-size:13px"
            Foreground="secondary"
          />
          <div class="git-welcome-actions">
            <WinButton
              :Content="welcomeInitBusy ? i18n.t('git.repoSettingsInitBusy') : i18n.t('git.welcomeCreate')"
              Style="AccentButtonStyle"
              @click="welcomeCreateRepo"
              :IsEnabled="!welcomeInitBusy && !welcomeBusy"
            />
            <WinButton
              :Content="i18n.t('git.welcomeEnter')"
              @click="welcomeEnterRepo"
              :IsEnabled="!welcomeBusy && !welcomeInitBusy"
            />
          </div>
          <div v-if="welcomeInitBusy || welcomeBusy" class="git-welcome-busy">
            <WinProgressRing :IsActive="true" :IsIndeterminate="true" :Width="20" :Height="20" />
            <WinTextBlock
              :Text="welcomeInitBusy ? i18n.t('git.repoSettingsInitBusy') : i18n.t('git.repoSettingsBusy')"
              Style="font-size:12px;opacity:.7"
            />
          </div>
          <div v-if="welcomeError" class="git-welcome-error">
            {{ welcomeError }}
          </div>
        </div>
      </div>

      <template v-else>
      <div class="git-toolbar">
        <div class="git-repo-name">
          <AppIcon name="git" :size="18" />
          <WinTextBlock
            v-if="repoName"
            :Text="repoName"
            Style="font-size:14px;font-weight:600"
          />
          <WinTextBlock
            v-else
            :Text="i18n.t('git.repoPlaceholder')"
            Style="font-size:14px"
            Foreground="secondary"
          />
        </div>
        <WinButton
          :Content="i18n.t('git.repoSettings')"
          @click="openRepoSettings"
          :IsEnabled="!loading"
        />
        <WinButton
          :Content="i18n.t('git.refresh')"
          @click="refreshAll"
          :IsEnabled="!loading && !repoError"
        />
        <div class="git-account-area">
          <WinTextBlock
            v-if="authState?.logged_in && authState?.user"
            :Text="i18n.t('git.loggedAs', { user: authState.user })"
            Style="font-size:12px"
            Foreground="secondary"
          />
          <WinTextBlock
            v-else
            :Text="i18n.t('git.notLoggedIn')"
            Style="font-size:12px"
            Foreground="secondary"
          />
          <WinButton
            :IsEnabled="!busy && !logoutBusy"
            @click="startLoginWizard"
            :Title="i18n.t('git.switchAccount')"
            Style="padding:4px;min-height:28px"
          >
            <AppIcon name="switchAccount" :size="16" />
          </WinButton>
          <WinButton
            v-if="authState?.logged_in"
            :IsEnabled="!busy && !logoutBusy"
            @click="logoutGithub"
            :Title="logoutBusy ? i18n.t('git.loggingOut') : i18n.t('git.logout')"
            Style="padding:4px;min-height:28px"
          >
            <AppIcon name="logout" :size="16" />
          </WinButton>
        </div>
      </div>

      <div v-if="repoError === 'norepo'" class="git-card git-warn">
        <WinTextBlock :Text="i18n.t('git.noRepo')" Style="color:var(--system-error, #c42b1c)" />
      </div>
      <div v-else-if="repoError" class="git-card git-warn">
        <WinTextBlock :Text="repoError" Style="color:var(--system-error, #c42b1c)" />
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
              :Content="i18n.t('git.commitAndPush')"
              Style="AccentButtonStyle"
              @click="openCommitPushDialog"
              :IsEnabled="!busy && stagedFiles.length > 0"
            />
            <WinButton
              :Content="i18n.t('git.push')"
              @click="push"
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
          <div
            v-for="c in commits"
            :key="c.hash"
            class="git-commit-item"
            :class="{ expanded: expandedCommit === c.hash }"
          >
            <div class="git-commit-row">
              <button
                class="git-commit-expand-btn"
                :aria-expanded="expandedCommit === c.hash"
                @click="toggleCommitExpand(c)"
                :title="i18n.t('git.commitExpand')"
              >
                <span class="git-commit-chevron" :class="{ rotated: expandedCommit === c.hash }">&#xE76C;</span>
              </button>
              <span class="git-commit-hash">{{ c.short_hash }}</span>
              <span class="git-commit-msg" @click="toggleCommitExpand(c)">{{ c.message }}</span>
              <span class="git-commit-meta">{{ c.author }} · {{ c.date }}</span>
            </div>
            <div v-if="expandedCommit === c.hash" class="git-commit-detail">
              <div class="git-commit-detail-section">
                <div class="git-commit-detail-label">{{ i18n.t('git.commitHash') }}</div>
                <div class="git-commit-detail-value git-commit-detail-hash">{{ c.hash }}</div>
              </div>
              <div class="git-commit-detail-section">
                <div class="git-commit-detail-label">{{ i18n.t('git.commitAuthor') }}</div>
                <div class="git-commit-detail-value">{{ c.author }}</div>
              </div>
              <div class="git-commit-detail-section">
                <div class="git-commit-detail-label">{{ i18n.t('git.commitDate') }}</div>
                <div class="git-commit-detail-value">{{ c.date }}</div>
              </div>
              <div v-if="c.body" class="git-commit-detail-section">
                <div class="git-commit-detail-label">{{ i18n.t('git.commitBodyLabel') }}</div>
                <pre class="git-commit-detail-body">{{ c.body }}</pre>
              </div>
              <div v-else class="git-commit-detail-empty">{{ i18n.t('git.commitBodyEmpty') }}</div>
              <div class="git-commit-detail-actions">
                <WinButton
                  :Content="i18n.t('git.revert')"
                  @click="openRevertDialog(c)"
                  :IsEnabled="!busy"
                  Style="font-size:12px"
                />
              </div>
            </div>
          </div>
        </section>
      </template>
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
      v-model:IsOpen="logoutConfirmDialog"
      :Title="i18n.t('git.logoutConfirmTitle')"
      :Content="i18n.t('git.logoutConfirmContent')"
      :PrimaryButtonText="i18n.t('git.logoutConfirmOk')"
      :CloseButtonText="i18n.t('git.logoutConfirmCancel')"
      DefaultButton="Close"
      @PrimaryButtonClick="confirmLogout"
    />

    <WinContentDialog
      v-model:IsOpen="resultDialog"
      :Title="resultDialogTitle"
      :Content="resultDialogContent"
      :CloseButtonText="i18n.t('git.resultClose')"
    />

    <WinContentDialog
      v-model:IsOpen="revertDialog"
      :Title="i18n.t('git.revertTitle')"
      :PrimaryButtonText="i18n.t('git.revertConfirm')"
      :CloseButtonText="i18n.t('git.revertCancel')"
      :IsPrimaryButtonEnabled="!revertBusy"
      DefaultButton="Close"
      @PrimaryButtonClick="confirmRevert"
    >
      <div class="git-revert-body">
        <div class="git-revert-info">
          <div><strong>{{ revertTarget?.short_hash }}</strong></div>
          <div class="git-revert-msg">{{ revertTarget?.message }}</div>
          <div class="git-revert-meta">{{ revertTarget?.author }} · {{ revertTarget?.date }}</div>
        </div>
        <WinCheckBox
          v-model="revertNoCommit"
          :Content="i18n.t('git.revertNoCommit')"
        />
        <div v-if="revertError" class="git-revert-error">
          {{ revertError }}
        </div>
        <div v-if="revertBusy" class="git-revert-busy">
          <WinProgressRing :IsActive="true" :IsIndeterminate="true" :Width="20" :Height="20" />
          <WinTextBlock :Text="i18n.t('git.revertBusy')" Style="font-size:12px;opacity:.7" />
        </div>
      </div>
    </WinContentDialog>

    <WinContentDialog
      v-model:IsOpen="repoSettingsDialog"
      :Title="i18n.t('git.repoSettingsTitle')"
      :PrimaryButtonText="repoSettingsNotGit ? i18n.t('git.repoSettingsInit') : i18n.t('git.repoSettingsConfirm')"
      :SecondaryButtonText="repoSettingsNotGit ? i18n.t('git.repoSettingsReselect') : ''"
      :CloseButtonText="i18n.t('git.repoSettingsCancel')"
      :IsPrimaryButtonEnabled="
        repoSettingsNotGit
          ? !repoSettingsInitBusy && !repoSettingsBusy && !!repoSettingsPath.trim()
          : !repoSettingsBusy && !cloning && !!repoSettingsPath.trim()
      "
      DefaultButton="Primary"
      @PrimaryButtonClick="repoSettingsNotGit ? initRepoAndEnterWizard() : confirmRepoSettings()"
      @SecondaryButtonClick="resetRepoSettingsSelection"
    >
      <div class="git-repo-settings-body">
        <div class="git-repo-settings-row">
          <label>{{ i18n.t('git.repoSettingsPath') }}</label>
          <div class="git-repo-settings-input-row">
            <WinTextBox
              :Text="repoSettingsPath"
              :PlaceholderText="i18n.t('git.repoPlaceholder')"
              @update:Text="(v: string) => (repoSettingsPath = v)"
              Style="flex:1"
            />
            <WinButton
              :Content="i18n.t('git.browse')"
              @click="browseRepoSettings"
              :IsEnabled="!repoSettingsBusy && !cloning"
            />
          </div>
        </div>

        <div class="git-repo-settings-divider"></div>

        <div class="git-repo-settings-row">
          <label>{{ i18n.t('git.cloneSection') }}</label>
          <div class="git-repo-settings-input-row">
            <WinTextBox
              :Text="cloneUrl"
              :PlaceholderText="i18n.t('git.cloneUrlPlaceholder')"
              @update:Text="(v: string) => (cloneUrl = v)"
              Style="flex:1"
            />
          </div>
          <div class="git-repo-settings-input-row">
            <WinTextBox
              :Text="cloneTarget"
              :PlaceholderText="i18n.t('git.cloneTargetPlaceholder')"
              @update:Text="(v: string) => (cloneTarget = v)"
              Style="flex:1"
            />
            <WinButton
              :Content="i18n.t('git.browse')"
              @click="browseCloneTarget"
              :IsEnabled="!cloning && !repoSettingsBusy"
            />
            <WinButton
              :Content="i18n.t('git.clone')"
              Style="AccentButtonStyle"
              @click="cloneRepo"
              :IsEnabled="!cloning && !repoSettingsBusy && !!cloneUrl && !!cloneTarget"
            />
          </div>
        </div>

        <div v-if="repoSettingsError" class="git-repo-settings-error">
          {{ repoSettingsError }}
        </div>
        <div v-if="repoSettingsBusy || cloning || repoSettingsInitBusy" class="git-repo-settings-busy">
          <WinProgressRing :IsActive="true" :IsIndeterminate="true" :Width="20" :Height="20" />
          <WinTextBlock
            :Text="
              repoSettingsInitBusy
                ? i18n.t('git.repoSettingsInitBusy')
                : cloning
                  ? i18n.t('git.cloning')
                  : i18n.t('git.repoSettingsBusy')
            "
            Style="font-size:12px;opacity:.7"
          />
        </div>
      </div>
    </WinContentDialog>

    <WinContentDialog
      v-model:IsOpen="commitPushDialog"
      :Title="i18n.t('git.commitPushTitle')"
      :PrimaryButtonText="commitPushOnly ? i18n.t('git.commitPushConfirmOnly') : i18n.t('git.commitPushConfirm')"
      :CloseButtonText="i18n.t('git.commitPushCancel')"
      :IsPrimaryButtonEnabled="canCommitPush"
      DefaultButton="Primary"
      @PrimaryButtonClick="confirmCommitPush"
    >
      <div class="git-commit-push-body">
        <div class="git-commit-push-row">
          <label>{{ i18n.t('git.commitMessage') }}</label>
          <WinTextBox
            :Text="commitPushMsg"
            :PlaceholderText="i18n.t('git.commitPlaceholder')"
            @update:Text="(v: string) => (commitPushMsg = v)"
            Style="width:100%"
          />
        </div>
        <div class="git-commit-push-row">
          <label>{{ i18n.t('git.commitBody') }}</label>
          <WinTextBox
            :Text="commitPushBody"
            :PlaceholderText="i18n.t('git.commitBodyPlaceholder')"
            :AcceptsReturn="true"
            @update:Text="(v: string) => (commitPushBody = v)"
            Style="width:100%;min-height:80px"
          />
        </div>
        <WinCheckBox
          v-model="commitPushOnly"
          :Content="i18n.t('git.commitPushOnly')"
        />
        <div v-if="commitPushError" class="git-commit-push-error">
          {{ commitPushError }}
        </div>
        <div v-if="commitPushBusy" class="git-commit-push-busy">
          <WinProgressRing :IsActive="true" :IsIndeterminate="true" :Width="20" :Height="20" />
          <WinTextBlock :Text="i18n.t('git.commitPushBusy')" Style="font-size:12px;opacity:.7" />
        </div>
      </div>
    </WinContentDialog>

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

        <div class="wiz-logs">
          <div v-if="!loginLogs.length && !loginError" class="wiz-log-empty">
            {{ i18n.t('git.wizLogsEmpty') }}
          </div>
          <div v-for="(line, i) in loginLogs" :key="i" class="wiz-log-line">{{ line }}</div>
          <div v-if="loginError" class="wiz-log-line wiz-log-error">{{ loginError }}</div>
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

.git-welcome {
  display: flex;
  flex-direction: column;
  gap: 24px;
  padding: 12px 0 40px;
}

.git-welcome-account {
  display: flex;
  align-items: center;
  gap: 8px;
  justify-content: flex-end;
  padding: 4px 8px;
  border-bottom: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.06));
  padding-bottom: 10px;
}

html.theme-dark .git-welcome-account {
  border-bottom-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.07));
}

.git-welcome-hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  padding: 32px 20px;
  border-radius: 12px;
  background: var(--LayerFillColorDefaultBrush, rgba(255, 255, 255, 0.5));
  border: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.08));
}

html.theme-dark .git-welcome-hero {
  background: var(--LayerFillColorDefaultBrush, rgba(255, 255, 255, 0.03));
  border-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.07));
}

.git-welcome-icon {
  width: 72px;
  height: 72px;
  border-radius: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--AccentButtonBackground, #005fb8);
  background: color-mix(in srgb, var(--AccentButtonBackground, #005fb8) 14%, transparent);
}

html.theme-dark .git-welcome-icon {
  color: #4cc2ff;
  background: color-mix(in srgb, #4cc2ff 16%, transparent);
}

.git-welcome-actions {
  display: flex;
  gap: 12px;
  margin-top: 6px;
  flex-wrap: wrap;
  justify-content: center;
}

.git-welcome-busy {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
}

.git-welcome-error {
  font-size: 12px;
  color: var(--system-error, #c42b1c);
  word-break: break-word;
  text-align: center;
  max-width: 460px;
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

.git-account-area {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-left: auto;
  padding-left: 12px;
  border-left: 1px solid var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.08));
}

html.theme-dark .git-account-area {
  border-left-color: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.08));
}

.git-account-area .win-btn {
  padding: 4px 8px;
  min-height: 28px;
  line-height: 1;
}

.git-repo-name {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
  padding: 4px 8px;
  border-radius: 6px;
  background: var(--ControlFillColorSecondaryBrush, rgba(0, 0, 0, 0.03));
  color: var(--AccentButtonBackground, #005fb8);
}

html.theme-dark .git-repo-name {
  background: var(--ControlFillColorSecondaryBrush, rgba(255, 255, 255, 0.05));
  color: #4cc2ff;
}

.git-repo-name .win-textblock {
  flex: 1;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
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

.git-commit-push-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 4px 0;
}

.git-commit-push-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.git-commit-push-row label {
  font-size: 13px;
  font-weight: 600;
  color: var(--TextFillColorPrimaryBrush, rgba(0, 0, 0, 0.9));
}

html.theme-dark .git-commit-push-row label {
  color: var(--TextFillColorPrimaryBrush, rgba(255, 255, 255, 0.9));
}

.git-commit-push-error {
  font-size: 12px;
  color: var(--system-error, #c42b1c);
  word-break: break-word;
}

.git-commit-push-busy {
  display: flex;
  align-items: center;
  gap: 8px;
}

.git-repo-settings-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 4px 0;
}

.git-repo-settings-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.git-repo-settings-row label {
  font-size: 13px;
  font-weight: 600;
  color: var(--TextFillColorPrimaryBrush, rgba(0, 0, 0, 0.9));
}

html.theme-dark .git-repo-settings-row label {
  color: var(--TextFillColorPrimaryBrush, rgba(255, 255, 255, 0.9));
}

.git-repo-settings-input-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.git-repo-settings-divider {
  height: 1px;
  margin: 4px 0;
  background: var(--CardStrokeColorDefaultBrush, rgba(0, 0, 0, 0.08));
}

html.theme-dark .git-repo-settings-divider {
  background: var(--CardStrokeColorDefaultBrush, rgba(255, 255, 255, 0.08));
}

.git-repo-settings-error {
  font-size: 12px;
  color: var(--system-error, #c42b1c);
  word-break: break-word;
}

.git-repo-settings-busy {
  display: flex;
  align-items: center;
  gap: 8px;
}

.git-commit-item {
  border-radius: 6px;
  transition: background 0.15s ease;
}

.git-commit-item + .git-commit-item {
  margin-top: 2px;
}

.git-commit-item.expanded {
  background: var(--ControlFillColorSecondaryBrush, rgba(0, 0, 0, 0.04));
}

html.theme-dark .git-commit-item.expanded {
  background: var(--ControlFillColorSecondaryBrush, rgba(255, 255, 255, 0.05));
}

.git-commit-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  font-size: 13px;
}

.git-commit-expand-btn {
  flex-shrink: 0;
  width: 24px;
  height: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  cursor: pointer;
  border-radius: 4px;
  color: var(--TextFillColorSecondaryBrush, rgba(0, 0, 0, 0.6));
  font-family: "WinUIOnWebIcons", "Segoe Fluent Icons", "Segoe MDL2 Assets", monospace;
  font-size: 12px;
  transition: background 0.12s ease;
}

.git-commit-expand-btn:hover {
  background: var(--ControlFillColorTertiaryBrush, rgba(0, 0, 0, 0.06));
}

html.theme-dark .git-commit-expand-btn {
  color: var(--TextFillColorSecondaryBrush, rgba(255, 255, 255, 0.6));
}

html.theme-dark .git-commit-expand-btn:hover {
  background: var(--ControlFillColorTertiaryBrush, rgba(255, 255, 255, 0.08));
}

.git-commit-chevron {
  display: inline-block;
  transition: transform 0.18s ease;
}

.git-commit-chevron.rotated {
  transform: rotate(90deg);
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
  cursor: pointer;
  user-select: none;
}

.git-commit-meta {
  flex-shrink: 0;
  font-size: 12px;
  opacity: 0.65;
}

.git-commit-detail {
  padding: 8px 12px 12px 40px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.git-commit-detail-section {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.git-commit-detail-label {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--TextFillColorSecondaryBrush, rgba(0, 0, 0, 0.6));
}

html.theme-dark .git-commit-detail-label {
  color: var(--TextFillColorSecondaryBrush, rgba(255, 255, 255, 0.6));
}

.git-commit-detail-value {
  font-size: 13px;
  word-break: break-word;
}

.git-commit-detail-hash {
  font-family: "Cascadia Code", "Consolas", monospace;
  color: var(--AccentButtonBackground, #005fb8);
}

html.theme-dark .git-commit-detail-hash {
  color: #4cc2ff;
}

.git-commit-detail-body {
  margin: 0;
  padding: 8px 10px;
  font-family: "Cascadia Code", "Consolas", monospace;
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--ControlFillColorDefaultBrush, rgba(0, 0, 0, 0.03));
  border-radius: 6px;
  color: var(--TextFillColorPrimaryBrush, rgba(0, 0, 0, 0.9));
}

html.theme-dark .git-commit-detail-body {
  background: var(--ControlFillColorDefaultBrush, rgba(255, 255, 255, 0.03));
  color: var(--TextFillColorPrimaryBrush, rgba(255, 255, 255, 0.9));
}

.git-commit-detail-empty {
  font-size: 12px;
  opacity: 0.55;
  font-style: italic;
}

.git-commit-detail-actions {
  display: flex;
  gap: 8px;
  margin-top: 4px;
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

.wiz-logs {
  max-height: 240px;
  overflow-y: auto;
  padding: 10px 12px;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.04);
  font-family: "Cascadia Code", "Consolas", monospace;
  font-size: 12px;
  line-height: 1.5;
}

html.theme-dark .wiz-logs {
  background: rgba(0, 0, 0, 0.3);
  color: rgba(255, 255, 255, 0.85);
}

.wiz-log-empty {
  opacity: 0.55;
  font-style: italic;
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
