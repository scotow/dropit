String.prototype.toTitleCase = function() {
    return this.charAt(0).toUpperCase() + this.substr(1);
};

document.addEventListener('DOMContentLoaded', documentReady, false);

function documentReady() {
    const orange = [0xff, 0xc6, 0x1d];
    const green = [0x15, 0xb1, 0x54];

    class File {
        constructor(fileRef) {
            this.fileRef = fileRef;
        }

        appendToDocument() {
            this.file = document.createElement('div');
            this.file.classList.add('file');
            
            const name = document.createElement('div');
            name.classList.add('name');
            name.innerText = this.fileRef.name;
            this.file.appendChild(name);

            this.progressBar = document.createElement('div');
            this.progressBar.classList.add('progress-bar');
            this.file.appendChild(this.progressBar);

            document.querySelector('.files').appendChild(this.file);
        }

        startUpload() {
            const req = new XMLHttpRequest();
            req.open('POST', '/', true);
            req.setRequestHeader('X-Filename', this.fileRef.name);
            req.setRequestHeader('Content-Type', this.fileRef.type);
            req.responseType = 'json';

            req.upload.onprogress = (event) => {
                const progress = event.loaded / event.total;
                this.progressBar.style.backgroundColor = `rgb(${(green[0] - orange[0]) * progress + orange[0]}, ${(green[1] - orange[1]) * progress + orange[1]}, ${(green[2] - orange[2]) * progress + orange[2]})`;
                this.progressBar.style.width = `${progress * 100}%`;
            };
            req.onload = (event) => {
                if (req.status === 201) {
                    setTimeout(() => {
                        this.uploadSucceeded(req.response);
                    }, 1200);
                } else {
                    this.file.classList.add('error');
                    this.progressBar.style.backgroundColor = '#ff5d24';
                    this.progressBar.style.width = '100%';
                    setTimeout(() => {
                        this.uploadFailed(req.response);
                    }, 1200);
                }
            };
            req.send(this.fileRef);
        }

        uploadSucceeded(data) {
            this.link = document.createElement('div');
            this.link.classList.add('link');
            this.link.innerText = data.link.short;
            this.progressBar.replaceWith(this.link);

            const info = document.createElement('div');
            info.classList.add('info');
            
            const qrcode = document.createElement('div');
            qrcode.classList.add('qrcode', 'hidden');
            new QRCode(qrcode, {
                text: data.link.short,
                width: 128,
                height: 128,
                colorDark : '#131313',
                colorLight : '#15b154',
                correctLevel : QRCode.CorrectLevel.L
            });
            qrcode.onclick = () => {
                qrcode.classList.toggle('hidden');
            };

            const right = document.createElement('div');
            right.classList.add('right');

            const details = document.createElement('div');
            details.classList.add('details');

            const size = document.createElement('div');
            size.classList.add('item');
            const sizeLabel = document.createElement('div');
            sizeLabel.classList.add('label');
            sizeLabel.innerText = 'Size';
            const sizeContent = document.createElement('div');
            sizeContent.title = 'Click to toggle between formats';
            sizeContent.classList.add('content', 'clickable');
            sizeContent.innerText = data.size.readable;
            let sizeFormat = 'readable';
            sizeContent.addEventListener('click', () => {
                switch (sizeFormat) {
                    case 'readable':
                        sizeContent.innerText = `${data.size.bytes.toLocaleString()}B`;
                        sizeFormat = 'bytes';
                        break;
                    case 'bytes':
                        sizeContent.innerText = data.size.readable;
                        sizeFormat = 'readable';
                        break;
                }
            });

            const longAlias = document.createElement('div');
            longAlias.classList.add('item');
            const longAliasLabel = document.createElement('div');
            longAliasLabel.classList.add('label');
            longAliasLabel.innerText = 'Long alias';
            const longAliasContent = document.createElement('div');
            longAlias.title = 'Click to copy to clipboard';
            longAliasContent.classList.add('content', 'clickable', 'copy');
            longAliasContent.setAttribute('data-clipboard-text', data.link.long);
            longAliasContent.innerText = data.alias.long;
            
            const expiration = document.createElement('div');
            expiration.classList.add('item');
            const expirationLabel = document.createElement('div');
            expirationLabel.classList.add('label', 'clickable');
            expirationLabel.innerText = 'Duration';
            const expirationContent = document.createElement('div');
            expirationContent.classList.add('content', 'clickable');
            expirationContent.innerText = data.expiration.duration.readable;
            let expirationFormat = 'duration';
            [expirationLabel, expirationContent].forEach((elem) => {
                elem.title = 'Click to toggle between formats';
                elem.addEventListener('click', () => {
                    switch (expirationFormat) {
                        case 'duration':
                            expirationLabel.innerText = 'Expiration'
                            expirationContent.innerText = data.expiration.date.readable;
                            expirationFormat = 'date';
                            break;
                        case 'date':
                            expirationLabel.innerText = 'Duration'
                            expirationContent.innerText = data.expiration.duration.readable;
                            expirationFormat = 'duration';
                            break;
                    }
                });
            });

            const actions = document.createElement('div');
            actions.classList.add('actions');
            
            const revokeButton = document.createElement('div');
            revokeButton.classList.add('button', 'warning', 'no-select');
            revokeButton.innerText = 'Revoke';

            const copyLinkButton = document.createElement('div');
            copyLinkButton.classList.add('button', 'success', 'no-select', 'copy');
            copyLinkButton.setAttribute('data-clipboard-text', data.link.short);
            copyLinkButton.innerText = 'Copy Link';

            info.append(qrcode, right);
            right.append(details, actions);
            details.append(size, longAlias, expiration);
            size.append(sizeLabel, sizeContent);
            longAlias.append(longAliasLabel, longAliasContent);
            expiration.append(expirationLabel, expirationContent);
            actions.append(/*revokeButton,*/ copyLinkButton);

            this.file.append(info);
        }

        uploadFailed(data) {
            const error = document.createElement('div');
            error.classList.add('error-message');
            error.innerText = data.error.toTitleCase();
            this.progressBar.replaceWith(error);

            const remove = document.createElement('div');
            remove.classList.add('button', 'error', 'no-select');
            remove.innerText = 'Remove';
            remove.addEventListener('click', () => {
                this.file.remove();
                if (document.querySelector('.files').childElementCount === 0) {
                    document.body.classList.remove('has-files');
                    document.querySelector('.form-files').classList.remove('visible');
                }
            })
            error.after(remove);
        }
    }

    function uploadFiles(files) {
        for (const f of files) {
            if (!f.name) continue;

            document.body.classList.add('has-files');
            setTimeout(() => {
                document.querySelector('.form-files').classList.add('visible');
            }, 1000);

            const file = new File(f);
            file.appendToDocument();
            file.startUpload();
        }
    }

    // Form input.
    document.querySelectorAll('input[type=file]').forEach((elem) => {
        elem.addEventListener('change', (event) => {
            uploadFiles(event.target.files);
        });
    });

    // Drag and drop.
    ['drag', 'dragstart', 'dragend', 'dragover', 'dragenter', 'dragleave', 'drop'].forEach((name) => {
        document.body.addEventListener(name, event => {
            event.preventDefault();
            event.stopPropagation();
        })
    });
    ['dragover', 'dragenter'].forEach((name) => {
        document.body.addEventListener(name, event => {
            document.body.classList.add('dragging');
        })
    });
    ['dragleave', 'dragend', 'drop'].forEach((name) => {
        document.body.addEventListener(name, event => {
            document.body.classList.remove('dragging');
        })
    });
    document.body.addEventListener('drop', (event) => {
        const files = [];
        for (let item of event.dataTransfer.items) {
            if (item.kind !== 'file') continue;
            if (item.webkitGetAsEntry && item.webkitGetAsEntry().isDirectory) continue;
            files.push(item.getAsFile());
        }
        uploadFiles(files);
    });

    new ClipboardJS('.copy');
}