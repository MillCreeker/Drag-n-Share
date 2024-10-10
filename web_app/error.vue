<template>
    <div class="text-center">
        <h1 class="text-9xl font-bold tracking-wide text-yellow-500">{{ error.statusCode }}</h1>
        <p class="text-3xl font-medium tracking-widest text-orange-500 mb-16">{{ error.message }}</p>
        <NuxtLink to="/"
            class="mt-4 p-2 rounded-lg text-white font-bold tracking-wide bg-gray-500 hover:bg-gray-700 active:bg-gray-900">
            Fly Back</NuxtLink>
    </div>
</template>

<script setup>
defineProps(['error']);
</script>