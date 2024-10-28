# JUnit

The JUnit report is a standard XML report that can be used with any CI/CD tool that supports JUnit reports. You can download this report by making a GET request to the proxy like this:
```http
GET http://localhost:3000/_ovp/junit
```

If you're using `curl` you could save this output to a file like:
```sh
curl -o junit.xml http://localhost:3000/_ovp/junit
```

References:
- [Official JUnit user guide](https://junit.org/junit5/docs/current/user-guide)
- [JUnit Report Examples](https://github.com/testmoapp/junitxml)
