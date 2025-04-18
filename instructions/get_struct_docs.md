We need to update the way in which find a struct, as the modules it's in are in the url. So let's isolate and re-use some of the functionality from the crate-items tool, which will list all the items and give a url for each, so we can find the matching struct, and then navigate to the struct web page that way.

Make test cases so that with the input TracerProviderBuilder for the opentelemetry_sdk crate it successfully works by getting the docs at: https://docs.rs/opentelemetry_sdk/latest/opentelemetry_sdk/trace/struct.TracerProviderBuilder.html

Please ensure that if provided trace::TracerProviderBuilder that also works.