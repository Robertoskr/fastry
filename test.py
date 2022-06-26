from fastry import Fastry


app = Fastry(__name__, settings={"debug":True})


def route(request):
    return "hello"

app.register("/somenice/path", route)