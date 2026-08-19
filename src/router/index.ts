import { createRouter, createWebHashHistory } from "vue-router";
import type { RouteRecordRaw } from "vue-router";

const routes: RouteRecordRaw[] = [
  { path: "/", redirect: "/home" },
  {
    path: "/home",
    name: "home",
    component: () => import("@/views/HomeView.vue"),
  },
  {
    path: "/diskclean",
    name: "diskclean",
    component: () => import("@/views/DiskCleanView.vue"),
  },
  {
    path: "/performance",
    name: "performance",
    component: () => import("@/views/PerformanceView.vue"),
  },
  {
    path: "/process",
    name: "process",
    component: () => import("@/views/ProcessView.vue"),
  },
  {
    path: "/optimize",
    name: "optimize",
    component: () => import("@/views/OptimizeView.vue"),
  },
  {
    path: "/visual",
    name: "visual",
    component: () => import("@/views/VisualView.vue"),
  },
  {
    path: "/git",
    name: "git",
    component: () => import("@/views/GitView.vue"),
  },
  {
    path: "/settings",
    name: "settings",
    component: () => import("@/views/SettingsView.vue"),
  },
  { path: "/:pathMatch(.*)*", redirect: "/home" },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

export default router;
