<script setup lang="ts">
import AppShell from "../components/shell/AppShell.vue";
import TenantDetail from "../components/tenants/TenantDetail.vue";
import { useRoute } from "vue-router";

const route = useRoute();
const id = route.params.id as string;
</script>

<template>
  <AppShell>
    <TenantDetail :id="id" />
  </AppShell>
</template>
