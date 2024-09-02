<template>
    <div>
        <p>{{ error.statusCode }}</p>
        <p>{{ error.message }}</p>
        <button @click="handleClearError">Go Back</button>
    </div>
</template>

<script setup>
    const { error } = defineProps(['error']);

    const handleClearError = () => clearError({ redirect: '/' });
</script>