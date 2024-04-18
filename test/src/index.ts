import express from "express";
import fs from "fs";
import bodyParser from "body-parser";

const app = express();

app.use(bodyParser.raw({ type: "application/xml" }));

const initXml = `<?xml version="1.0" encoding="UTF-8"?>
<config-auth client="vpn" type="auth-request">
    <version who="test">0.1(1)</version>
    <auth id="main">
        <message>Please enter your username.</message>
        <form method="post" action="/auth">
            <input type="text" name="username" label="Username:"/>
        </form>
    </auth>
</config-auth>`;

const askPasswordXml = `<?xml version="1.0" encoding="UTF-8"?>
<config-auth client="vpn" type="auth-request">
    <version who="test">0.1(1)</version>
    <auth id="main">
        <message>Please enter your password.</message>
        <form method="post" action="/auth">
            <input type="password" name="password" label="Password:"/>
        </form>
    </auth>
</config-auth>`;

const successXml = `<?xml version="1.0" encoding="UTF-8"?>
<config-auth client="vpn" type="complete">
<version who="test">0.1(1)</version>
<auth id="success">
<title>SSL VPN Service</title></auth>
<config client="vpn" type="private"><vpn-profile-manifest><vpn rev="1.0"><file type="profile" service-type="user"><uri>/profiles/profile.xml</uri><hash type="sha1">123</hash></file></vpn></vpn-profile-manifest>
</config></config-auth>`;

const httpsOptions = {
  key: fs.readFileSync("./key.pem"),
  cert: fs.readFileSync("./cert.pem"),
};

app.post("*", (req, res) => {
  console.log(req.headers);

  const body = req.body.toString("utf-8");
  console.log(body);

  res.header("set-cookie", "webvpncontext=; path=/; Secure");
  res.header("content-type", "text/xml");
  res.header("x-transcend-version", "1");

  let xml = initXml;

  if (body.includes("username")) {
    xml = askPasswordXml;
  }

  if (body.includes("password")) {
    xml = successXml;
    res.header("set-cookie", "webvpncontext=123; Secure");
    res.header("set-cookie", "webvpn=123; Secure");
    res.header("set-cookie", "webvpnc=; path=/; Secure");
    res.header("set-cookie", "webvpnc=bu:123; path=/; Secure");
  }

  res.send(xml);
});

const server = require("https").createServer(httpsOptions, app);

server.listen(3000, () => {
  console.log("Server is running on port 3000");
});

/* test for client */

import axios from "axios";

const tryConnect = async ({
  name,
  pass,
  url,
}: {
  name: string;
  pass: string;
  url: string;
}) => {
  const headers = {
    "content-type": "application/xml",
    "user-agent": "AnyConnect-compatible OpenConnect VPN Agent v9.12",
    accept: "*/*",
    "accept-encoding": "identity",
    "x-transcend-version": "1",
    "x-aggregate-auth": "1",
    "x-support-http-auth": "1",
    "x-anyconnect-strap-pubkey":
      "testpubkey1",
    "x-anyconnect-strap-dh-pubkey":
      "testpubkey2",
  };

  const res1 = await axios.request({
    url,
    method: "POST",
    headers,
    data: "",
  });

  console.log(res1.data);
  console.log();

  const res2 = await axios.request({
    url,
    method: "POST",
    headers,
    data: `<?xml version="1.0" encoding="UTF-8"?>
    <config-auth client="vpn" type="auth-reply" aggregate-auth-version="2">
        <version who="vpn">v9.12</version>
        <device-id>mac-intel</device-id>
        <capabilities>
            <auth-method>single-sign-on-v2</auth-method>
            <auth-method>single-sign-on-external-browser</auth-method>
        </capabilities>
        <auth>
            <username>${name}</username>
        </auth>
    </config-auth>`,
  });

  console.log(res2.data);
  console.log();

  await new Promise((resolve) => setTimeout(resolve, 3000));

  const res3 = await axios.request({
    url,
    method: "POST",
    headers,
    data: `<?xml version="1.0" encoding="UTF-8"?>
    <config-auth client="vpn" type="auth-reply" aggregate-auth-version="2">
        <version who="vpn">v9.12</version>
        <device-id>mac-intel</device-id>
        <capabilities>
            <auth-method>single-sign-on-v2</auth-method>
            <auth-method>single-sign-on-external-browser</auth-method>
        </capabilities>
        <auth>
            <password>${pass}</password>
        </auth>
    </config-auth>`,
  });

  console.log("=====================================");
  console.log(res3.headers["set-cookie"]);
  console.log("=====================================");
  console.log(res3.data);
};
