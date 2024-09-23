<template>
    <div class="text-center">
        <p>{{ error.statusCode }}</p>
        <p>{{ error.message }}</p>
        <NuxtLink to="/">Go Back</NuxtLink>
    </div>
</template>

<script setup>
defineProps(['error']);
</script>