String.prototype.toTitleCase = function() {
    return this.charAt(0).toUpperCase() + this.substr(1);
};

String.prototype.plural = function(n) {
    return n >= 2 ? `${this}s` : this;
};

document.addEventListener('DOMContentLoaded', documentReady, false);

function documentReady() {
    class File {
        constructor(fileRef) {
            this.fileRef = fileRef;
        }

        buildBase(filename) {
            this.file = document.createElement('div');
            this.file.classList.add('file');
            
            const name = document.createElement('div');
            name.classList.add('name');
            name.innerText = filename;
            this.file.append(name);

            document.querySelector('.files').append(this.file);
        }

        startUpload() {
            const data = { uploaded: false };
            files.add(data);

            this.progressBar = document.createElement('div');
            this.progressBar.classList.add('progress-bar');
            this.file.append(this.progressBar);

            const req = new XMLHttpRequest();
            req.open('POST', '/', true);
            req.setRequestHeader('X-Filename', encodeURIComponent(this.fileRef.name));
            req.setRequestHeader('Content-Type', this.fileRef.type);
            req.responseType = 'json';

            req.upload.onprogress = (event) => {
                const progress = event.loaded / event.total;
                this.progressBar.style.backgroundColor = `rgb(${(0x15 - 0xff) * progress + 0xff}, ${(0xb1 - 0xc6) * progress + 0xc6}, ${(0x54 - 0x1d) * progress + 0x1d})`;
                this.progressBar.style.width = `${progress * 100}%`;
            };
            req.onload = (event) => {
                if (req.status === 201) {
                    data.uploaded = true;
                    const resp = req.response; 
                    for (const key in resp) {
                        if (key === 'success') continue;
                        data[key] = resp[key];
                    }
                    files.save();

                    setTimeout(() => {
                        this.buildDetails(data);
                    }, 1200);
                } else {
                    this.file.classList.add('error');
                    this.progressBar.style.backgroundColor = '#ff5d24';
                    this.progressBar.style.width = '100%';

                    files.remove(data);
                    setTimeout(() => {
                        this.buildError(req.response);
                    }, 1200);
                }
            };
            req.send(this.fileRef);
        }

        buildDetails(data) {
            const link = document.createElement('div');
            link.classList.add('link', 'selectable');
            link.innerText = data.link.short;

            const info = document.createElement('div');
            info.classList.add('info');

            const qrcodeWrapper = document.createElement('div');
            qrcodeWrapper.classList.add('qrcode', 'hidden');
            const qrcode = new QRCode(qrcodeWrapper, {
                text: data.link.short,
                width: 128,
                height: 128,
                colorDark : '#131313',
                colorLight : 'TEMPLATE_COLOR',
                correctLevel : QRCode.CorrectLevel.L
            });
            qrcodeWrapper.onclick = () => {
                qrcodeWrapper.classList.toggle('hidden');
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
                        sizeContent.innerText = `${data.size.bytes.toLocaleString()} B`;
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
            longAliasContent.classList.add('content', 'selectable');
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
            function updateExpirationLabel() {
                switch (expirationFormat) {
                    case 'date':
                        expirationLabel.innerText = 'Expiration'
                        expirationContent.innerText = data.expiration.date.readable;
                        break;
                    case 'duration':
                        expirationLabel.innerText = 'Duration'
                        expirationContent.innerText = data.expiration.duration.readable;
                        break;
                }
            }
            [expirationLabel, expirationContent].forEach((elem) => {
                elem.title = 'Click to toggle between formats';
                elem.addEventListener('click', (event) => {
                    expirationFormat = expirationFormat === 'date' ? 'duration' : 'date';
                    updateExpirationLabel();
                })
            });

            const bottom = document.createElement('div');
            bottom.classList.add('bottom');

            const actions = document.createElement('div');
            actions.classList.add('actions');

            const copyShort = document.createElement('div');
            copyShort.classList.add('copy-short', 'clickable', 'copy');
            copyShort.setAttribute('data-clipboard-text', data.link.short);
            copyShort.innerText = 'Copy Link';

            const dropdown = document.createElement('div');
            dropdown.classList.add('dropdown', 'clickable');

            const menu = document.createElement('div');
            menu.classList.add('menu');

            dropdown.addEventListener('click', (event) => {
                menu.classList.toggle('opened');
                event.stopPropagation();
            });

            const download = document.createElement('div');
            download.classList.add('item');
            download.innerText = 'Download';
            download.addEventListener('click', () => {
                document.location = data.link.short;
            });

            const separator = document.createElement('div');
            separator.classList.add('separator');

            const copyLong = document.createElement('div');
            copyLong.classList.add('item', 'copy');
            copyLong.setAttribute('data-clipboard-text', data.link.long);
            copyLong.innerText = 'Copy long link';

            const newAlias = document.createElement('div');
            newAlias.classList.add('item', 'sub-menu');
            newAlias.innerText = 'Generate new alias';

            const aliasMenu = document.createElement('div');
            aliasMenu.classList.add('menu');

            for (const t of ['short', 'long', 'both']) {
                const type = document.createElement('div');
                type.classList.add('item');
                type.innerText = t.toTitleCase();
                type.addEventListener('click', () => {
                    if (confirm(`Generating new ${t === 'both' ? 'aliases' : 'alias'} will make all people with a current link unable to access it. Confirm?`)) {
                        const req = new XMLHttpRequest();
                        let path = `/${data.alias.short}/aliases`;
                        if (t !== 'both') {
                            path = path.concat(`/${t}`);
                        }
                        req.open('PATCH', path, true);
                        req.setRequestHeader('Authorization', data.admin);
                        req.responseType = 'json';
                        req.onload = (event) => {
                            if (req.status === 200) {
                                if (t === 'short' || t === 'both') {
                                    data.alias.short = req.response.alias.short;
                                    data.link.short = req.response.link.short;
                                    link.innerText = data.link.short;
                                    copyShort.setAttribute('data-clipboard-text', data.link.short);
                                    qrcode.makeCode(data.link.short);
                                }
                                if (t === 'long' || t === 'both') {
                                    data.alias.long = req.response.alias.long;
                                    data.link.long = req.response.link.long;
                                    longAliasContent.innerText = data.alias.long;
                                    copyLong.setAttribute('data-clipboard-text', data.link.long);
                                }
                                files.save();
                            } else {
                                alert(`An error occured while trying to generate new ${t === 'both' ? 'aliases' : 'alias'}: ${req.response.error}.`);
                            }
                        }
                        req.send();
                    };
                });
                aliasMenu.append(type);
            }

            const extend = document.createElement('div');
            extend.classList.add('item');
            extend.innerText = 'Extend duration';
            extend.addEventListener('click', () => {
                if (confirm('Extending this file will try to reset its duration to its initial one, which will still count toward your quota. Confirm?')) {
                    const req = new XMLHttpRequest();
                    req.open('PATCH', `/${data.alias.short}/expiration`, true);
                    req.setRequestHeader('Authorization', data.admin);
                    req.responseType = 'json';
                    req.onload = (event) => {
                        if (req.status === 200) {
                            delete req.response.success;
                            data.expiration = req.response;
                            files.save();
                            updateExpirationLabel();
                        } else {
                            alert(`An error occured while trying to extend expiration: ${req.response.error}.`);
                        }
                    };
                    req.send();
                }
            });

            const downloads = document.createElement('div');
            downloads.classList.add('item', 'sub-menu');
            downloads.innerText = 'Limit downloads';

            const downloadsMenu = document.createElement('div');
            downloadsMenu.classList.add('menu');

            for (const n of [1, 3, 5, 10, 25, 100, 0]) {
                const count = document.createElement('div');
                count.classList.add('item');
                count.innerText = n ? `${n} ${'download'.plural(n)}` : 'Unlimited';
                count.addEventListener('click', () => {
                    const req = new XMLHttpRequest();
                    req.open('PATCH', `/${data.alias.short}/downloads/${n}`, true);
                    req.setRequestHeader('Authorization', data.admin);
                    req.responseType = 'json';
                    req.onload = (event) => {
                        if (req.status === 200) {
                        } else {
                            alert(`An error occured while trying to set downloads limit: ${req.response.error}.`);
                        }
                    };
                    req.send();
                });
                downloadsMenu.append(count);
            }

            const forget = document.createElement('div');
            forget.classList.add('item', 'warning');
            forget.innerText = 'Forget';
            forget.addEventListener('click', (event) => {
                if (confirm('Forgetting this file will still count toward your quota. Confirm?')) {
                    this.file.remove();
                    fileListUpdated();
                    files.remove(data);
                }
            });

            const revoke = document.createElement('div');
            revoke.classList.add('item', 'destructive');
            revoke.innerText = 'Revoke';
            revoke.addEventListener('click', () => {
                if (confirm('Revoking this file will make all people with a link unable to access it. Confirm?')) {
                    const req = new XMLHttpRequest();
                    req.open('DELETE', `/${data.alias.short}`, true);
                    req.setRequestHeader('Authorization', data.admin);
                    req.responseType = 'json';
                    req.onload = (event) => {
                        if (req.status === 200) {
                            this.file.remove();
                            fileListUpdated();
                            files.remove(data);
                        } else {
                            alert(`An error occured while trying to revoke this file: ${req.response.error}.`);
                        }
                    };
                    req.send();
                }
            });

            info.append(qrcodeWrapper, right);
            right.append(details, bottom);
            details.append(size, longAlias, expiration);
            size.append(sizeLabel, sizeContent);
            longAlias.append(longAliasLabel, longAliasContent);
            expiration.append(expirationLabel, expirationContent);
            bottom.append(actions);
            actions.append(copyShort, dropdown, menu);
            newAlias.append(aliasMenu);
            downloads.append(downloadsMenu);
            menu.append(download, separator.cloneNode(), copyLong, newAlias, separator, extend, downloads, separator.cloneNode(), forget, revoke);

            if (this.progressBar) this.progressBar.remove();
            this.file.append(link, info);
        }

        buildError(data) {
            const remove = document.createElement('div');
            remove.classList.add('remove', 'clickable', 'no-select');
            remove.addEventListener('click', () => {
                this.file.remove();
                fileListUpdated();
            })

            const error = document.createElement('div');
            error.classList.add('error-message');
            error.innerText = data.error.toTitleCase();

            if (this.progressBar) this.progressBar.remove();
            this.file.append(remove, error);
        }
    }

    class Files {
        constructor() {
            this.files = JSON.parse(localStorage.getItem('files') || '[]');
            for (const data of this.valids()) {
                const file = new File(null);
                file.buildBase(data.name);
                file.buildDetails(data);
            }
            this.updateArchiveButton(this.files);
            fileListUpdated(true);
        }

        add(file) {
            this.files.push(file);
            this.save();
        }

        remove(file) {
            const index = this.files.indexOf(file);
            if (index === -1) return;
            this.files.splice(index, 1);
            this.save();
        }

        save() {
            const valids = this.valids();
            localStorage.setItem('files', JSON.stringify(valids));
            this.updateArchiveButton(valids);
        }

        valids() {
            const now = (new Date()).getTime() / 1000;
            return this.files.filter((f) => f.uploaded && f.expiration.date.timestamp > now);
        }

        updateArchiveButton(files) {
            document.querySelector('.archive-link').setAttribute('data-clipboard-text', `${window.location.origin}/${files.map(f => f.alias.short).join('+')}`);
        }
    }

    function uploadFiles(files) {
        for (const f of files) {
            if (!f.name) continue;
            const file = new File(f);
            file.buildBase(f.name);
            file.startUpload();
        }
        fileListUpdated(false);
    }

    function fileListUpdated(fast) {
        const fileCount = document.querySelector('.files').childElementCount;
        if (fileCount >= 1) {
            document.body.classList.add('has-files');
            setTimeout(() => {
                document.querySelector('.form-files').classList.add('visible');
            }, fast ? 0 : 1000);
        } else {
            document.body.classList.remove('has-files');
            document.querySelector('.form-files').classList.remove('visible');
            document.querySelector('.archive-link').classList.remove('visible');
        }
        if (fileCount >= 2) {
            setTimeout(() => {
                document.querySelector('.archive-link').classList.add('visible');
            }, fast ? 0 : 1000);
        } else {
            document.querySelector('.archive-link').classList.remove('visible');
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

    document.addEventListener('click', (event) => {
        const opened = document.querySelector('.menu.opened');
        if (opened) {
            opened.classList.remove('opened');
        }
    });

    new ClipboardJS('.copy');
    const files = new Files();
}