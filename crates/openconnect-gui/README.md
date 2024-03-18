# Openconnect GUI

## Resolve damaged application on MacOS

The problem is caused by code signing. The application is not signed with a valid certificate. To resolve this issue, you can follow the following steps to self-sign the application and create a dmg file.

- Created a certificate for code signing according to this [link](https://stackoverflow.com/questions/27474751/how-can-i-codesign-an-app-without-being-in-the-mac-developer-program)

- Sign the application under `target/release/bundled/macos/openconnect-gui.app`

- Create a dmg file

  - Install `create-dmg` with `brew install create-dmg`

  - Run the following script under `target/release/bundled/macos`
    ```bash
    create-dmg \
    --volname "Openconnect GUI" \
    --window-pos 200 120 \
    --window-size 800 400 \
    --icon-size 100 \
    --icon "openconnect-gui.app" 200 190 \
    --hide-extension "openconnect-gui.app" \
    --app-drop-link 600 185 \
    "Openconnect GUI.dmg" \
    "openconnect-gui.app/"
    ```
  - The dmg file has been created and you can distribute it to your users.
