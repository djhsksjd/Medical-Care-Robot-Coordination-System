#include <bits/stdc++.h>



//actuall this should be the queue of the robot or the task
template <typename T>
class t_task_queue
{
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
            //we get the 
        }

        void push_process()
        {

        }

        std::optional<T> try_pop()
        {
            
        }

        std::optional<T> pop_process()
        {

        }

        










    private:
        t_task_queue() = default//set the constructor to the default/ private
        //this let cannot create new instance outside 

        T tq[N];
        int size;
        int lp, rp;

        //or we use the stl
        mutable std::mutex m_;
        std::queue<T> t_q;




};
