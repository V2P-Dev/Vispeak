#include <QString>
#include <kwin/config-kwin.h>
#include <iostream>

int main()
{
    std::cout << QString(KWIN_VERSION_STRING).toStdString() << '\n';
}
