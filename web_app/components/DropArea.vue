<template>
    <div class="flex justify-end mr-8 mt-4">
        <i v-if="!isServerOnline" class="material-icons text-orange-500">wifi_off</i>
        <i v-else class="material-icons text-orange-500">wifi</i>
    </div>
    <div id="drop-area" class="rounded-lg border-dashed border-2 p-4 m-4 mt-0 text-center"
        :class="{ 'border-yellow-500': isFileHovering, 'border-orange-500': !isFileHovering }">
        <slot></slot>
    </div>
</template>

<script setup>
let isFileHovering = ref(false);

let isServerOnline = ref(false);

$fetch('http://api.localhost/hmm').then((results) => {
    isServerOnline.value = true;
});

onMounted(() => {
    const dropArea = document.getElementById('drop-area');

    dropArea.addEventListener('dragenter', (e) => {
        e.preventDefault();
        e.stopPropagation();
        isFileHovering.value = true;
    });

    dropArea.addEventListener('dragleave', (e) => {
        e.preventDefault();
        e.stopPropagation();

        if (isFileInContainer(e)) {
            return;
        }

        isFileHovering.value = false;
    });

    dropArea.addEventListener('drop', (e) => {
        e.preventDefault();
        e.stopPropagation();

        // const files = e.dataTransfer.files;
        // const dT = new DataTransfer();

        // for (let i = 0; i < files.length; i++) {
        //     const file = files[i];
        //     dT.items.add(file);
        // }

        // const workableFiles = dT.files;
        // console.log(files);

        // const base64 = toBase64(workableFiles[0]).then((b64) => {
        //     console.log(b64);
        // });

        isFileHovering.value = false;
    });

    function isFileInContainer(e) {
        let isInContainer = true;
        const rect = dropArea.getBoundingClientRect();

        if (e.clientY < rect.top ||
            e.clientY >= rect.bottom ||
            e.clientX < rect.left ||
            e.clientX >= rect.right) {
            isInContainer = false
        }

        return isInContainer;
    };

    function toBase64(file) {
        return new Promise((resolve, reject) => {
            const reader = new FileReader();
            reader.readAsDataURL(file);
            reader.onload = () => resolve(reader.result);
            reader.onerror = error => reject(error);
        })
    };

    window.addEventListener('dragover', (e) => {
        e.preventDefault();
        e.stopPropagation();
    });
    window.addEventListener('drop', (e) => {
        e.preventDefault();
        e.stopPropagation();
    });
});

</script>