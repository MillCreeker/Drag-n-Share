export async function convertFiles(files) {
    const fileArr = [];

    const toBase64 = file => new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.readAsDataURL(file);
        reader.onload = () => resolve(reader.result);
        reader.onerror = error => reject(error);
    });
    try {
        for (let i = 0; i < files.length; i++) {
            const file = files[i];

            const base64 = await toBase64(file);
            const fileString = `name:${file.name};${base64}`;

            fileArr.push({
                name: file.name,
                size: file.size,
                data: fileString
            });
        }
    } catch (e) {
        console.error(e);
    }

    return fileArr;
}