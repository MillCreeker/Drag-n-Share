<template>
    <div>
        <button>
            <i class="material-icons">description</i>
            {{ filename }}
        </button>
    </div>
</template>

<script setup>
    const { filename } = defineProps(['filename']);
</script>