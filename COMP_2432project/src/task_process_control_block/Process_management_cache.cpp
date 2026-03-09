#include <bits/stdc++.h>
#include <thread>
#include <mutex>


//actuall this should be the queue of the robot or the task
template <typename T>
class t_task_queue
{
    //this queue is one for each type
    const int N = 100010;

    public:

        //this means the singleton class
        static t_task_queue& instance()
        {
            static t_task_queue inst;
            return inst;
        }

        //delete the Copy constructor and assignment operator
        t_task_queue(const t_task_queue&) = delete;
        t_task_queue& operator= (const t_task_queue&) = delete;

        void push(T v)
        {
            std::lock_guard<std::mutex> lock(m_);
            //we get the T type use std::move to transfer the resource into the queue
            t_q.push(std::move(v));
        }

        void push_process()
        {

        }

        std::optional<T> try_pop()
        {
            std::lock_guard<std::mutex> lock(m_);
            if(t_q.empty()){
                return std::nullopt;
            }
            T ret = std::move(t_q.front());
            t_q.pop();
            return ret;
        }

        std::optional<T> pop_process()
        {

        }

        bool empty() const
        {
            std::lock_guard<std::mutex> lock(m_);
            return t_q.empty();
        }
        

    private:
        t_task_queue() = default;//set the constructor to the default/ private
        //this let cannot create new instance outside 

        // T tq[N];
        // int size;
        // int lp, rp;

        //or we use the stl
        mutable std::mutex m_;
        std::queue<T> t_q;


};













// signed main()
// {
//      // 1) Push some work items
//     t_task_queue<std::string>::instance().push("task A");
//     t_task_queue<std::string>::instance().push("task B");

//     // 2) Pop items
//     while (!t_task_queue<std::string>::instance().empty()) {
//         auto item = t_task_queue<std::string>::instance().try_pop();
//         if (item) {
//             std::cout << "Got: " << *item << "\n";
//         }
//     }

//     // 3) (Optional) Producer/consumer example
//     std::thread producer([] {
//         for (int i = 0; i < 5; i++) {
//             t_task_queue<int>::instance().push(i);
//         }
//     });

//     std::thread consumer([] {
//         int got = 0;
//         while (got < 5) {
//             auto v = t_task_queue<int>::instance().try_pop();
//             if (v) {
//                 std::cout << "Consumed: " << *v << "\n";
//                 got++;
//             }
//             // simple busy-wait; in real code you’d use a condition_variable
//         }
//     });

//     producer.join();
//     consumer.join();
// }
