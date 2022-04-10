document.getElementById('login-form').addEventListener('submit', (event) => {
    event.preventDefault();

    const req = new XMLHttpRequest();
    req.open('POST', '/auth', true);
    req.responseType = 'json';
    req.onload = (_event) => {
        if (req.status === 201) {
            document.cookie = `session=${req.response.token}; path=/; max-age=${60*60*24*28}`;
            window.location = '/';
        } else {
            alert(`An error occurred while login in: ${req.response.error}.`);
            console.error(`An error occurred while login in: ${req.response.error}.`);
        }
    };
    req.send(JSON.stringify(
        {
            username: document.getElementById('username').value,
            password: document.getElementById('password').value,
        }
    ));
})